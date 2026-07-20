/* tslint:disable */
/* eslint-disable */

/**
 * Convert a single colour from one model to another, applying per-channel
 * `Math.round` to numeric results (matches `color-convert`'s public wrapper).
 *
 * Returns a JS array for numeric models, a string for `hex`/`keyword`, or a
 * number for `ansi16`/`ansi256`.
 *
 * # Errors
 *
 * Returns a JS `Error` if `from`/`to` are unknown model names, the input
 * does not match the `from` model, or no conversion path exists.
 */
export function convert_route(from: string, to: string, input: any): any;

/**
 * Like [`convert_route`] but without per-channel rounding (matches the
 * `.raw` variant on every `color-convert` route).
 */
export function convert_route_raw(from: string, to: string, input: any): any;

export function hsl_to_rgb_batch(input: Float32Array): Float32Array;

export function hsv_to_rgb_batch(input: Float32Array): Float32Array;

export function rgb_to_cmyk_batch(input: Uint8Array): Float32Array;

export function rgb_to_hsl_batch(input: Uint8Array): Float32Array;

export function rgb_to_hsv_batch(input: Uint8Array): Float32Array;

export function rgb_to_lab_batch(input: Uint8Array): Float32Array;

export function rgb_to_oklab_batch(input: Uint8Array): Float32Array;

export function rgb_to_xyz_batch(input: Uint8Array): Float32Array;
