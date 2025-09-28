/* tslint:disable */
/* eslint-disable */
export function set_low_end_mode(enabled: boolean): void;
export function set_render_scale(scale: number): void;
export function set_performance_mode(mode: number): void;
export function wasm_init_canvas(width: number, height: number): void;
export function wasm_update_vu(buf: Uint8Array): void;
export function wasm_update_mouse(x: number, y: number, active: boolean, down: boolean): void;
export function frame_ptr(): number;
export function frame_len(): number;
export function wasm_set_screen_text(s: string): void;
export function wasm_render_frame(): void;

export type InitInput = RequestInfo | URL | Response | BufferSource | WebAssembly.Module;

export interface InitOutput {
  readonly memory: WebAssembly.Memory;
  readonly set_low_end_mode: (a: number) => void;
  readonly set_render_scale: (a: number) => void;
  readonly set_performance_mode: (a: number) => void;
  readonly wasm_init_canvas: (a: number, b: number) => void;
  readonly wasm_update_vu: (a: number, b: number) => void;
  readonly wasm_update_mouse: (a: number, b: number, c: number, d: number) => void;
  readonly frame_ptr: () => number;
  readonly frame_len: () => number;
  readonly wasm_set_screen_text: (a: number, b: number) => void;
  readonly wasm_render_frame: () => void;
  readonly __wbindgen_exn_store: (a: number) => void;
  readonly __externref_table_alloc: () => number;
  readonly __wbindgen_export_2: WebAssembly.Table;
  readonly __wbindgen_malloc: (a: number, b: number) => number;
  readonly __wbindgen_realloc: (a: number, b: number, c: number, d: number) => number;
  readonly __wbindgen_start: () => void;
}

export type SyncInitInput = BufferSource | WebAssembly.Module;
/**
* Instantiates the given `module`, which can either be bytes or
* a precompiled `WebAssembly.Module`.
*
* @param {{ module: SyncInitInput }} module - Passing `SyncInitInput` directly is deprecated.
*
* @returns {InitOutput}
*/
export function initSync(module: { module: SyncInitInput } | SyncInitInput): InitOutput;

/**
* If `module_or_path` is {RequestInfo} or {URL}, makes a request and
* for everything else, calls `WebAssembly.instantiate` directly.
*
* @param {{ module_or_path: InitInput | Promise<InitInput> }} module_or_path - Passing `InitInput` directly is deprecated.
*
* @returns {Promise<InitOutput>}
*/
export default function __wbg_init (module_or_path?: { module_or_path: InitInput | Promise<InitInput> } | InitInput | Promise<InitInput>): Promise<InitOutput>;
