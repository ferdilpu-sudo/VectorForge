import fs from "node:fs";

const css = fs.readFileSync(
  new URL("../src/styles/global.css", import.meta.url),
  "utf8",
);

function block(pattern, label) {
  const match = css.match(pattern);
  if (!match) throw new Error(`Missing CSS block: ${label}`);
  return match[1];
}

function variables(source) {
  return Object.fromEntries(
    [...source.matchAll(/--([a-z-]+):\s*(#[0-9a-f]{3,8})\s*;/gi)].map(
      ([, name, value]) => [name, value],
    ),
  );
}

function rgb(hex) {
  const value = hex.slice(1);
  const full =
    value.length === 3
      ? value
          .split("")
          .map((part) => part + part)
          .join("")
      : value.slice(0, 6);
  return [0, 2, 4].map((index) => parseInt(full.slice(index, index + 2), 16) / 255);
}

function luminance(hex) {
  const [r, g, b] = rgb(hex).map((channel) =>
    channel <= 0.03928
      ? channel / 12.92
      : ((channel + 0.055) / 1.055) ** 2.4,
  );
  return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

function ratio(a, b) {
  const first = luminance(a);
  const second = luminance(b);
  return (Math.max(first, second) + 0.05) / (Math.min(first, second) + 0.05);
}

const dark = variables(block(/:root\s*{([\s\S]*?)}/, "dark root"));
const light = {
  ...dark,
  ...variables(
    block(/:root\[data-theme="light"\]\s*{([\s\S]*?)}/, "light root"),
  ),
};

const checks = [];
for (const [theme, tokens] of [
  ["dark", dark],
  ["light", light],
]) {
  checks.push(
    [theme, "text/surface", tokens.text, tokens.surface, 4.5],
    [theme, "muted/surface", tokens.muted, tokens.surface, 4.5],
    [theme, "danger/surface", tokens["danger-text"], tokens.surface, 4.5],
    [theme, "warning/surface", tokens["warning-text"], tokens.surface, 4.5],
    [theme, "focus/surface", tokens.focus, tokens.surface, 3],
    [theme, "border/surface", tokens.border, tokens.surface, 3],
    [theme, "onPrimary/primary", "#ffffff", tokens.primary, 4.5],
  );
}

let failed = false;
for (const [theme, label, foreground, background, minimum] of checks) {
  const value = ratio(foreground, background);
  const pass = value >= minimum;
  failed ||= !pass;
  console.log(
    `${pass ? "PASS" : "FAIL"} ${theme.padEnd(5)} ${label.padEnd(20)} ${value.toFixed(
      2,
    )}:1 (min ${minimum}:1)`,
  );
}

if (failed) process.exitCode = 1;
