// Terminal theme presets. Each entry maps cleanly to xterm.js's
// `ITheme` shape so the Terminal component can hand it straight to
// the constructor / `term.options.theme = …` without reshaping.
//
// We intentionally keep these as static objects (not generated) so a
// reader can audit the palette at a glance. Eight presets feels like
// a sweet spot — covers ~95% of preferences without becoming a long
// dropdown that nobody reads to the end of.

export interface TerminalTheme {
  background: string;
  foreground: string;
  cursor: string;
  cursorAccent: string;
  selectionBackground: string;

  black: string;
  red: string;
  green: string;
  yellow: string;
  blue: string;
  magenta: string;
  cyan: string;
  white: string;

  brightBlack: string;
  brightRed: string;
  brightGreen: string;
  brightYellow: string;
  brightBlue: string;
  brightMagenta: string;
  brightCyan: string;
  brightWhite: string;
}

export interface ThemePreset {
  id: string;
  name: string;
  /// One-word vibe to help users pick without trying every one.
  hint: string;
  theme: TerminalTheme;
}

const claudedeck: ThemePreset = {
  id: "claudedeck",
  name: "ClaudeDeck",
  hint: "Brand · orange on near-black",
  theme: {
    background: "#0e0e10",
    foreground: "#e8e8ea",
    cursor: "#ff8c42",
    cursorAccent: "#0e0e10",
    selectionBackground: "#3a3a4a",
    black: "#1c1c23", red: "#e35d6a", green: "#5fbf6f", yellow: "#e0c25c",
    blue: "#5fb5d8", magenta: "#b48bff", cyan: "#5fb5d8", white: "#e8e8ea",
    brightBlack: "#6c6c78", brightRed: "#ff7785", brightGreen: "#7fdc8f",
    brightYellow: "#ffd97a", brightBlue: "#7fc8e8", brightMagenta: "#c8a8ff",
    brightCyan: "#7fc8e8", brightWhite: "#ffffff",
  },
};

const dracula: ThemePreset = {
  id: "dracula",
  name: "Dracula",
  hint: "Purple/pink high-contrast classic",
  theme: {
    background: "#282a36",
    foreground: "#f8f8f2",
    cursor: "#bbbbbb",
    cursorAccent: "#282a36",
    selectionBackground: "#44475a",
    black: "#21222c", red: "#ff5555", green: "#50fa7b", yellow: "#f1fa8c",
    blue: "#bd93f9", magenta: "#ff79c6", cyan: "#8be9fd", white: "#f8f8f2",
    brightBlack: "#6272a4", brightRed: "#ff6e6e", brightGreen: "#69ff94",
    brightYellow: "#ffffa5", brightBlue: "#d6acff", brightMagenta: "#ff92df",
    brightCyan: "#a4ffff", brightWhite: "#ffffff",
  },
};

const oneDark: ThemePreset = {
  id: "one-dark",
  name: "One Dark",
  hint: "VSCode default · warm accents",
  theme: {
    background: "#282c34",
    foreground: "#abb2bf",
    cursor: "#528bff",
    cursorAccent: "#282c34",
    selectionBackground: "#3e4451",
    black: "#282c34", red: "#e06c75", green: "#98c379", yellow: "#e5c07b",
    blue: "#61afef", magenta: "#c678dd", cyan: "#56b6c2", white: "#abb2bf",
    brightBlack: "#5c6370", brightRed: "#e06c75", brightGreen: "#98c379",
    brightYellow: "#e5c07b", brightBlue: "#61afef", brightMagenta: "#c678dd",
    brightCyan: "#56b6c2", brightWhite: "#ffffff",
  },
};

const tokyoNight: ThemePreset = {
  id: "tokyo-night",
  name: "Tokyo Night",
  hint: "Soft midnight blues",
  theme: {
    background: "#1a1b26",
    foreground: "#c0caf5",
    cursor: "#c0caf5",
    cursorAccent: "#1a1b26",
    selectionBackground: "#283457",
    black: "#15161e", red: "#f7768e", green: "#9ece6a", yellow: "#e0af68",
    blue: "#7aa2f7", magenta: "#bb9af7", cyan: "#7dcfff", white: "#a9b1d6",
    brightBlack: "#414868", brightRed: "#f7768e", brightGreen: "#9ece6a",
    brightYellow: "#e0af68", brightBlue: "#7aa2f7", brightMagenta: "#bb9af7",
    brightCyan: "#7dcfff", brightWhite: "#c0caf5",
  },
};

const catppuccin: ThemePreset = {
  id: "catppuccin-mocha",
  name: "Catppuccin Mocha",
  hint: "Pastel · warm",
  theme: {
    background: "#1e1e2e",
    foreground: "#cdd6f4",
    cursor: "#f5e0dc",
    cursorAccent: "#1e1e2e",
    selectionBackground: "#585b70",
    black: "#45475a", red: "#f38ba8", green: "#a6e3a1", yellow: "#f9e2af",
    blue: "#89b4fa", magenta: "#f5c2e7", cyan: "#94e2d5", white: "#bac2de",
    brightBlack: "#585b70", brightRed: "#f38ba8", brightGreen: "#a6e3a1",
    brightYellow: "#f9e2af", brightBlue: "#89b4fa", brightMagenta: "#f5c2e7",
    brightCyan: "#94e2d5", brightWhite: "#a6adc8",
  },
};

const solarizedDark: ThemePreset = {
  id: "solarized-dark",
  name: "Solarized Dark",
  hint: "Calm browns · easy on the eyes",
  theme: {
    background: "#002b36",
    foreground: "#839496",
    cursor: "#93a1a1",
    cursorAccent: "#002b36",
    selectionBackground: "#073642",
    black: "#073642", red: "#dc322f", green: "#859900", yellow: "#b58900",
    blue: "#268bd2", magenta: "#d33682", cyan: "#2aa198", white: "#eee8d5",
    brightBlack: "#586e75", brightRed: "#cb4b16", brightGreen: "#586e75",
    brightYellow: "#657b83", brightBlue: "#839496", brightMagenta: "#6c71c4",
    brightCyan: "#93a1a1", brightWhite: "#fdf6e3",
  },
};

const githubDark: ThemePreset = {
  id: "github-dark",
  name: "GitHub Dark",
  hint: "Familiar · professional",
  theme: {
    background: "#0d1117",
    foreground: "#c9d1d9",
    cursor: "#c9d1d9",
    cursorAccent: "#0d1117",
    selectionBackground: "#264f78",
    black: "#484f58", red: "#ff7b72", green: "#3fb950", yellow: "#d29922",
    blue: "#58a6ff", magenta: "#bc8cff", cyan: "#39c5cf", white: "#b1bac4",
    brightBlack: "#6e7681", brightRed: "#ffa198", brightGreen: "#56d364",
    brightYellow: "#e3b341", brightBlue: "#79c0ff", brightMagenta: "#d2a8ff",
    brightCyan: "#56d4dd", brightWhite: "#f0f6fc",
  },
};

const nord: ThemePreset = {
  id: "nord",
  name: "Nord",
  hint: "Glacier blues · minimalist",
  theme: {
    background: "#2e3440",
    foreground: "#d8dee9",
    cursor: "#d8dee9",
    cursorAccent: "#2e3440",
    selectionBackground: "#434c5e",
    black: "#3b4252", red: "#bf616a", green: "#a3be8c", yellow: "#ebcb8b",
    blue: "#81a1c1", magenta: "#b48ead", cyan: "#88c0d0", white: "#e5e9f0",
    brightBlack: "#4c566a", brightRed: "#bf616a", brightGreen: "#a3be8c",
    brightYellow: "#ebcb8b", brightBlue: "#81a1c1", brightMagenta: "#b48ead",
    brightCyan: "#8fbcbb", brightWhite: "#eceff4",
  },
};

export const THEME_PRESETS: ThemePreset[] = [
  claudedeck, dracula, oneDark, tokyoNight, catppuccin, solarizedDark, githubDark, nord,
];

export function getPreset(id: string): ThemePreset {
  return THEME_PRESETS.find((p) => p.id === id) ?? claudedeck;
}

/// Apply opacity to a theme's background by converting hex → rgba.
/// xterm.js accepts both hex and rgba in the `background` field, so we
/// just generate the rgba form when opacity < 1.
export function withOpacity(theme: TerminalTheme, opacity: number): TerminalTheme {
  if (opacity >= 0.999) return theme;
  const rgba = hexToRgba(theme.background, opacity);
  return { ...theme, background: rgba };
}

function hexToRgba(hex: string, alpha: number): string {
  const m = hex.replace("#", "");
  const bigint = parseInt(m, 16);
  const r = (bigint >> 16) & 255;
  const g = (bigint >> 8) & 255;
  const b = bigint & 255;
  return `rgba(${r}, ${g}, ${b}, ${alpha.toFixed(3)})`;
}
