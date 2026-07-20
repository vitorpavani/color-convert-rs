declare const convert: Convert;

export = convert;

interface RouteFn {
  (...args: number[] | string[] | [string]): number[] | string | number;
  raw: (...args: number[] | string[] | [string]) => number[] | string | number;
}

interface Model {
  channels: number;
  labels: string[] | string;
  rgb: RouteFn;
  hsl: RouteFn;
  hsv: RouteFn;
  hwb: RouteFn;
  cmyk: RouteFn;
  xyz: RouteFn;
  lab: RouteFn;
  lch: RouteFn;
  oklab: RouteFn;
  oklch: RouteFn;
  hex: RouteFn;
  keyword: RouteFn;
  ansi16: RouteFn;
  ansi256: RouteFn;
  hcg: RouteFn;
  apple: RouteFn;
  gray: RouteFn;
}

interface Convert {
  rgb: Model;
  hsl: Model;
  hsv: Model;
  hwb: Model;
  cmyk: Model;
  xyz: Model;
  lab: Model;
  lch: Model;
  oklab: Model;
  oklch: Model;
  hex: Model;
  keyword: Model;
  ansi16: Model;
  ansi256: Model;
  hcg: Model;
  apple: Model;
  gray: Model;
}
