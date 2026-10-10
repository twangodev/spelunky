#!/usr/bin/env python3
"""Export official Classic assets deterministically, using Python's standard library.

The GmkSplitter extraction emits non-interlaced, 8-bit RGB/RGBA PNGs. Restrict the
decoder to those formats rather than silently converting unsupported source data.
"""

import argparse
import hashlib
import json
from pathlib import Path
import shutil
import struct
import xml.etree.ElementTree as ET
import zlib


PNG_SIGNATURE = b"\x89PNG\r\n\x1a\n"


def read_png(path):
    data = path.read_bytes()
    if data[:8] != PNG_SIGNATURE:
        raise ValueError(f"Not a PNG: {path}")
    offset, compressed = 8, bytearray()
    width = height = channels = None
    while offset < len(data):
        length = struct.unpack_from(">I", data, offset)[0]
        kind = data[offset + 4:offset + 8]
        payload = data[offset + 8:offset + 8 + length]
        crc = struct.unpack_from(">I", data, offset + 8 + length)[0]
        if zlib.crc32(kind + payload) & 0xFFFFFFFF != crc:
            raise ValueError(f"Invalid PNG CRC: {path}")
        if kind == b"IHDR":
            width, height, depth, color, compression, filtering, interlace = struct.unpack(">IIBBBBB", payload)
            if depth != 8 or color not in (2, 6) or compression or filtering or interlace:
                raise ValueError(f"Unsupported PNG encoding: {path}")
            channels = 4 if color == 6 else 3
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            break
        offset += 12 + length
    if width is None:
        raise ValueError(f"Missing PNG header: {path}")
    stride = width * channels
    raw = zlib.decompress(compressed)
    if len(raw) != (stride + 1) * height:
        raise ValueError(f"Invalid PNG pixel length: {path}")
    pixels = bytearray()
    previous = bytearray(stride)
    for y in range(height):
        start = y * (stride + 1)
        method = raw[start]
        row = bytearray(raw[start + 1:start + 1 + stride])
        for x in range(stride):
            left = row[x - channels] if x >= channels else 0
            above = previous[x]
            corner = previous[x - channels] if x >= channels else 0
            if method == 0:
                predictor = 0
            elif method == 1:
                predictor = left
            elif method == 2:
                predictor = above
            elif method == 3:
                predictor = (left + above) // 2
            elif method == 4:
                p = left + above - corner
                distances = abs(p - left), abs(p - above), abs(p - corner)
                predictor = (left, above, corner)[distances.index(min(distances))]
            else:
                raise ValueError(f"Invalid PNG filter {method}: {path}")
            row[x] = (row[x] + predictor) & 255
        if channels == 4:
            pixels.extend(row)
        else:
            for x in range(0, stride, 3):
                pixels.extend(row[x:x + 3] + b"\xff")
        previous = row
    return width, height, pixels


def png_bytes(width, height, pixels):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data) & 0xFFFFFFFF)
    rows = b"".join(b"\x00" + pixels[y * width * 4:(y + 1) * width * 4] for y in range(height))
    return PNG_SIGNATURE + chunk(b"IHDR", struct.pack(">IIBBBBB", width, height, 8, 6, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(rows, 9)) + chunk(b"IEND", b"")


def alpha_bounds(width, height, pixels, tolerance=0):
    xs, ys = [], []
    for y in range(height):
        for x in range(width):
            if pixels[(y * width + x) * 4 + 3] > tolerance:
                xs.append(x)
                ys.append(y)
    return [min(xs), min(ys), max(xs), max(ys)] if xs else None


def union_bounds(bounds):
    bounds = [b for b in bounds if b is not None]
    return [min(b[0] for b in bounds), min(b[1] for b in bounds), max(b[2] for b in bounds), max(b[3] for b in bounds)] if bounds else None


def sha256(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True, ensure_ascii=True) + "\n", encoding="utf-8")


def append_precise_masks(asset, images, collision):
    """Append portable row-major LSB-first alpha masks without atlas gutters."""
    if asset["mask"]["shape"] != "PRECISE":
        return
    separate = asset["mask"]["separate"]
    tolerance = asset["mask"]["alpha_tolerance"]
    groups = [[image] for image in images] if separate else [images]
    masks = []
    for group in groups:
        width = max(image[0] for image in group)
        height = max(image[1] for image in group)
        stride = (width + 7) // 8
        encoded = bytearray(stride * height)
        for image_width, image_height, pixels in group:
            for y in range(image_height):
                for x in range(image_width):
                    if pixels[(y * image_width + x) * 4 + 3] > tolerance:
                        encoded[y * stride + x // 8] |= 1 << (x % 8)
        bounds = union_bounds(alpha_bounds(w, h, pixels, tolerance) for w, h, pixels in group)
        masks.append({"mask_offset": len(collision), "mask_stride": stride, "mask_width": width, "mask_height": height, "mask_bounds": bounds})
        collision.extend(encoded)
    for index, frame in enumerate(asset["frames"]):
        frame.update(masks[index if separate else 0])


def verify_precise_masks(asset, images, collision, expected_offset):
    fields = {"mask_offset", "mask_stride", "mask_width", "mask_height", "mask_bounds"}
    if asset["mask"]["shape"] != "PRECISE":
        if any(fields.intersection(frame) for frame in asset["frames"]):
            raise ValueError("Non-precise frame unexpectedly has a bitmask")
        return expected_offset
    separate = asset["mask"]["separate"]
    tolerance = asset["mask"]["alpha_tolerance"]
    union_offset = expected_offset
    for index, frame in enumerate(asset["frames"]):
        group = [images[index]] if separate else images
        width, height = max(i[0] for i in group), max(i[1] for i in group)
        stride = (width + 7) // 8
        offset = expected_offset if separate else union_offset
        expected_metadata = {"mask_offset": offset, "mask_stride": stride, "mask_width": width, "mask_height": height, "mask_bounds": union_bounds(alpha_bounds(w, h, p, tolerance) for w, h, p in group)}
        if any(frame.get(key) != value or key not in frame for key, value in expected_metadata.items()):
            raise ValueError(f"Precise mask metadata mismatch: resource {asset['id']}, frame {index}")
        if offset < 0 or offset + stride * height > len(collision):
            raise ValueError(f"Precise mask outside collision.bin: resource {asset['id']}, frame {index}")
        # Check every stored bit independently against original alpha, including
        # padding bits beyond the image width (which must all remain zero).
        for y in range(height):
            for x in range(stride * 8):
                expected_bit = x < width and any(x < w and y < h and p[(y * w + x) * 4 + 3] > tolerance for w, h, p in group)
                actual_bit = bool(collision[offset + y * stride + x // 8] & (1 << (x % 8)))
                if actual_bit != expected_bit:
                    raise ValueError(f"Precise mask bit mismatch: resource {asset['id']}, frame {index}, pixel ({x}, {y})")
        if separate or index == 0:
            expected_offset += stride * height
    return expected_offset


def verify(source, original, output):
    manifest = json.loads((output / "manifest.json").read_text())
    for relative, digest in manifest["generated_files"].items():
        if sha256(output / relative) != digest:
            raise ValueError(f"Generated file hash mismatch: {relative}")
    for relative, digest in manifest["source_files"].items():
        if sha256(source / relative) != digest:
            raise ValueError(f"Source file hash mismatch: {relative}")
    for relative, digest in manifest["source_archives"].items():
        if sha256(original / relative) != digest:
            raise ValueError(f"Official archive hash mismatch: {relative}")
    metadata = json.loads((output / "atlas.json").read_text())
    width, height, pixels = read_png(output / "atlas.png")
    if (width, height) != (metadata["width"], metadata["height"]):
        raise ValueError("Atlas dimension mismatch")
    if pixels[:4] != b"\xff\xff\xff\xff":
        raise ValueError("Missing white texel")
    if metadata.get("collision_file") != "collision.bin" or metadata.get("collision_bit_order") != "lsb0":
        raise ValueError("Unsupported or missing precise collision mask format")
    collision = (output / "collision.bin").read_bytes()
    collision_offset = 0
    checked = 0
    for group in ("Sprites", "Backgrounds"):
        for path in sorted((source / group).rglob("*.xml")):
            if path.name.startswith("_"):
                continue
            root = ET.parse(path).getroot()
            if root.tag not in ("sprite", "background"):
                continue
            key = "sprites" if group == "Sprites" else "backgrounds"
            asset = metadata[key][path.stem]
            if asset["id"] != int(root.attrib["id"]):
                raise ValueError(f"Resource ID mismatch: {path}")
            if root.tag == "sprite":
                frame_paths = sorted(path.with_suffix(".images").glob("*.png"), key=lambda p: int(p.stem.split()[-1]))
                origin = root.find("origin")
                if asset["origin"] != [int(origin.attrib["x"]), int(origin.attrib["y"])]:
                    raise ValueError(f"Sprite origin mismatch: {path}")
                mask, bounds = root.find("mask"), root.find("mask/bounds")
                expected_mask = {"shape": mask.findtext("shape"), "separate": mask.findtext("separate") == "true", "alpha_tolerance": int(bounds.attrib.get("alphaTolerance", 0)), "bounds_mode": bounds.attrib["mode"]}
                if asset["mask"] != expected_mask:
                    raise ValueError(f"Collision mask metadata mismatch: {path}")
            else:
                frame_paths = [path.with_suffix(".png")]
            if len(frame_paths) != len(asset["frames"]):
                raise ValueError(f"Frame count mismatch: {path}")
            source_images = []
            for image_path, record in zip(frame_paths, asset["frames"]):
                frame_width, frame_height, expected = read_png(image_path)
                source_images.append((frame_width, frame_height, expected))
                if (frame_width, frame_height) != (record["w"], record["h"]):
                    raise ValueError(f"Frame dimension mismatch: {image_path}")
                if root.tag == "sprite" and record["bounds"] != alpha_bounds(frame_width, frame_height, expected, asset["mask"]["alpha_tolerance"]):
                    raise ValueError(f"Collision bounds mismatch: {image_path}")
                for row in range(frame_height):
                    target = ((record["y"] + row) * width + record["x"]) * 4
                    start = row * frame_width * 4
                    if pixels[target:target + frame_width * 4] != expected[start:start + frame_width * 4]:
                        raise ValueError(f"Atlas pixel mismatch: {image_path}, row {row}")
                checked += 1
            if root.tag == "sprite" and asset["bounds"] != union_bounds(frame["bounds"] for frame in asset["frames"]):
                raise ValueError(f"Combined collision bounds mismatch: {path}")
            if root.tag == "sprite":
                collision_offset = verify_precise_masks(asset, source_images, collision, collision_offset)
    if collision_offset != len(collision):
        raise ValueError("Trailing or missing precise collision bytes")
    audio = json.loads((output / "audio.json").read_text())["sounds"]
    for record in audio.values():
        if sha256(output / record["path"]) != sha256(original / "game/sound" / Path(record["path"]).name):
            raise ValueError(f"Audio mismatch: {record['path']}")
    print(f"Verified {checked} images pixel-for-pixel, origins/IDs, {len(collision)} precise collision bytes, {len(audio)} sounds, and all SHA-256 hashes")


def export(source, original, output, atlas_width):
    frames, sprites, backgrounds, source_hashes = [], {}, {}, {}
    collision = bytearray()
    for path in sorted((source / "Sprites").rglob("*.xml")):
        if path.name.startswith("_"):
            continue
        tree = ET.parse(path).getroot()
        if tree.tag != "sprite":
            continue
        origin = tree.find("origin")
        mask, bounds = tree.find("mask"), tree.find("mask/bounds")
        frame_paths = sorted(path.with_suffix(".images").glob("*.png"), key=lambda p: int(p.stem.split()[-1]))
        if not frame_paths:
            raise ValueError(f"Sprite has no frames: {path}")
        if path.stem in sprites:
            raise ValueError(f"Duplicate sprite name: {path.stem}")
        metadata = {
            "id": int(tree.attrib["id"]),
            "origin": [int(origin.attrib["x"]), int(origin.attrib["y"])],
            "frames": [],
            "mask": {"shape": mask.findtext("shape"), "separate": mask.findtext("separate") == "true", "alpha_tolerance": int(bounds.attrib.get("alphaTolerance", 0)), "bounds_mode": bounds.attrib["mode"]},
        }
        if metadata["mask"]["bounds_mode"] != "AUTO":
            raise ValueError(f"Unsupported source mask bounds mode: {path}")
        sprites[path.stem] = metadata
        source_hashes[path.relative_to(source).as_posix()] = sha256(path)
        sprite_images = []
        for index, image_path in enumerate(frame_paths):
            width, height, pixels = read_png(image_path)
            sprite_images.append((width, height, pixels))
            record = {"x": 0, "y": 0, "w": width, "h": height, "bounds": alpha_bounds(width, height, pixels, metadata["mask"]["alpha_tolerance"])}
            metadata["frames"].append(record)
            frames.append((f"sprite:{path.stem}:{index:05}", record, pixels))
            source_hashes[image_path.relative_to(source).as_posix()] = sha256(image_path)
        metadata["bounds"] = union_bounds(frame["bounds"] for frame in metadata["frames"])
        append_precise_masks(metadata, sprite_images, collision)
    for path in sorted((source / "Backgrounds").glob("*.xml")):
        if path.name.startswith("_"):
            continue
        tree = ET.parse(path).getroot()
        image_path = path.with_suffix(".png")
        width, height, pixels = read_png(image_path)
        record = {"x": 0, "y": 0, "w": width, "h": height}
        backgrounds[path.stem] = {"id": int(tree.attrib["id"]), "origin": [0, 0], "frames": [record], "use_as_tileset": tree.findtext("useAsTileset") == "true"}
        frames.append((f"background:{path.stem}", record, pixels))
        for item in (path, image_path):
            source_hashes[item.relative_to(source).as_posix()] = sha256(item)

    # Stable height-first shelf packing with a transparent one-pixel gutter.
    # Reserve the first scanline for a white texel used by untextured geometry.
    x, y, shelf_height = 0, 2, 0
    for name, record, pixels in sorted(frames, key=lambda f: (-f[1]["h"], -f[1]["w"], f[0])):
        width, height = record["w"], record["h"]
        if width + 2 > atlas_width:
            raise ValueError(f"Frame too wide for atlas: {name}")
        if x + width + 2 > atlas_width:
            x, y, shelf_height = 0, y + shelf_height, 0
        record["x"], record["y"] = x + 1, y + 1
        x += width + 2
        shelf_height = max(shelf_height, height + 2)
    atlas_height = 1 << (y + shelf_height - 1).bit_length()
    atlas = bytearray(atlas_width * atlas_height * 4)
    atlas[:4] = b"\xff\xff\xff\xff"
    for _, record, pixels in frames:
        for row in range(record["h"]):
            target = ((record["y"] + row) * atlas_width + record["x"]) * 4
            start = row * record["w"] * 4
            atlas[target:target + record["w"] * 4] = pixels[start:start + record["w"] * 4]

    output.mkdir(parents=True, exist_ok=True)
    (output / "atlas.png").write_bytes(png_bytes(atlas_width, atlas_height, atlas))
    (output / "collision.bin").write_bytes(collision)
    write_json(output / "atlas.json", {"version": 1, "width": atlas_width, "height": atlas_height, "white_pixel": [0, 0], "collision_file": "collision.bin", "collision_bit_order": "lsb0", "sprites": sprites, "backgrounds": backgrounds})
    audio = {}
    sound_dir = original / "game/sound"
    if not sound_dir.is_dir():
        raise ValueError(f"Missing official audio directory: {sound_dir}")
    (output / "audio").mkdir(exist_ok=True)
    for path in sorted(sound_dir.iterdir()):
        if path.is_file():
            relative = f"audio/{path.name}"
            shutil.copyfile(path, output / relative)
            audio[path.stem] = {"path": relative, "bytes": path.stat().st_size, "sha256": sha256(path)}
    write_json(output / "audio.json", {"version": 1, "sounds": audio})
    shutil.copyfile(original / "source/COPYING.txt", output / "COPYING.txt")
    shutil.copyfile(original / "source/readme.txt", output / "ORIGINAL_README.txt")
    archives = {p.name: sha256(p) for p in sorted(original.glob("*.zip"))}
    generated_paths = ["atlas.png", "atlas.json", "collision.bin", "audio.json", "COPYING.txt", "ORIGINAL_README.txt"] + [record["path"] for record in audio.values()]
    generated = {relative: sha256(output / relative) for relative in sorted(generated_paths)}
    write_json(output / "manifest.json", {"version": 1, "source_archives": archives, "source_files": source_hashes, "generated_files": generated, "counts": {"sprites": len(sprites), "sprite_frames": sum(len(s["frames"]) for s in sprites.values()), "backgrounds": len(backgrounds), "audio": len(audio), "collision_bytes": len(collision), "precise_masks": sum(len(s["frames"]) if s["mask"]["separate"] else 1 for s in sprites.values() if s["mask"]["shape"] == "PRECISE")}})
    print(f"Exported {len(sprites)} sprites, {len(frames)} images, {len(collision)} precise collision bytes, {len(audio)} sounds; atlas {atlas_width}x{atlas_height}")


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--source", type=Path, default=Path("references/classic-1.1"))
    parser.add_argument("--original", type=Path, default=Path("references/original"))
    parser.add_argument("--output", type=Path, default=Path("assets"))
    parser.add_argument("--atlas-width", type=int, default=2048)
    parser.add_argument("--verify", action="store_true", help="Verify existing output against every source image and audio file without writing files")
    args = parser.parse_args()
    if args.atlas_width < 2 or args.atlas_width & (args.atlas_width - 1):
        parser.error("--atlas-width must be a power of two greater than one")
    if args.verify:
        verify(args.source, args.original, args.output)
    else:
        export(args.source, args.original, args.output, args.atlas_width)


if __name__ == "__main__":
    main()
