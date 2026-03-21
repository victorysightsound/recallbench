interface Theme {
  "color-scheme": string
  "--color-base-100": string
  "--color-base-200": string
  "--color-base-300": string
  "--color-base-content": string
  "--color-primary": string
  "--color-primary-content": string
  "--color-secondary": string
  "--color-secondary-content": string
  "--color-accent": string
  "--color-accent-content": string
  "--color-neutral": string
  "--color-neutral-content": string
  "--color-info": string
  "--color-info-content": string
  "--color-success": string
  "--color-success-content": string
  "--color-warning": string
  "--color-warning-content": string
  "--color-error": string
  "--color-error-content": string
  "--radius-selector": string
  "--radius-field": string
  "--radius-box": string
  "--size-selector": string
  "--size-field": string
  "--border": string
  "--depth": string
  "--noise": string
}


interface Themes {
  blush: Theme
  citrus: Theme
  cyber: Theme
  cream: Theme
  dark: Theme
  depths: Theme
  dusk: Theme
  executive: Theme
  frost: Theme
  electric: Theme
  evergreen: Theme
  harvest: Theme
  honeycomb: Theme
  ice: Theme
  lavender: Theme
  light: Theme
  midnight: Theme
  mint: Theme
  mocha: Theme
  neon: Theme
  onyx: Theme
  paper: Theme
  pearl: Theme
  print: Theme
  pumpkin: Theme
  rose: Theme
  royal: Theme
  sage: Theme
  sketch: Theme
  slate: Theme
  sorbet: Theme
  starlight: Theme
  tidal: Theme
  vampire: Theme
  vintage: Theme
  [key: string]: Theme
}

declare const themes: Themes
export default themes