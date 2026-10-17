/// <reference types="vite/client" />

declare module 'virtual:spelunky' {
  export default function init(): Promise<unknown>;
  export function start(): void | Promise<void>;
}
