/** Vendored VersaTiles v6.1.1 styles; assets retain upstream tile attribution. */
const palettes = [
  { theme: "colorful", label: "Colorful" },
  { theme: "natural", label: "Natural" },
  { theme: "muted", label: "Muted" },
  { theme: "gray", label: "Gray" },
  { theme: "toner", label: "Toner" },
] as const;

export const VERSATILES_STYLE_PRESETS = palettes.flatMap(({ theme, label }) => [
  {
    id: `versatiles-${theme}` as const,
    label: `VersaTiles ${label}`,
    description: `Bản đồ ${label} với nền sáng và tên địa điểm bản địa.`,
    url: `/map-styles/versatiles/${theme}.json`,
    pitch: 0,
    provider: "VersaTiles",
  },
  {
    id: `versatiles-${theme}-dark` as const,
    label: `VersaTiles ${label} tối`,
    description: `Bản đồ ${label} với nền tối và tên địa điểm bản địa.`,
    url: `/map-styles/versatiles/${theme}-dark.json`,
    pitch: 0,
    provider: "VersaTiles",
  },
]);
