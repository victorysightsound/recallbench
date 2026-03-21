import { generateThemesObject } from "./functions/generateThemesObject.js"
import { generatePlugins } from "./functions/generatePlugins.js"
import { generateImports } from "./functions/generateImports.js"
import { copyFile } from "./functions/copyFile.js"

// Dev build: generates only the JS plugin files needed for @plugin usage.
// Skips raw CSS generation (generateRawStyles, packCss, generateChunks, generateThemes)
// which require postcss-selector-parser and produce standalone CSS bundles.
async function build() {
  try {
    console.time("core-ui build")

    // Phase 1: Generate plugin JS files from source CSS
    // Paths are relative to CWD (web/src/)
    await Promise.all([
      copyFile("./functions/themePlugin.js", "./theme/themePlugin.js", "index.js"),
      generatePlugins({ type: "base", srcDir: "themes", distDir: "theme" }),
      generatePlugins({ type: "base", srcDir: "base", distDir: "base", exclude: ["reset"] }),
      generatePlugins({ type: "component", srcDir: "components", distDir: "components" }),
      generatePlugins({ type: "utility", srcDir: "utilities", distDir: "utilities" }),
    ])

    // Phase 2: Generate imports.js and theme/object.js (depends on Phase 1 outputs)
    await Promise.all([
      generateImports("imports.js"),
      generateThemesObject("./theme/object.js"),
    ])

    console.timeEnd("core-ui build")
  } catch (error) {
    console.error("Build error:", error)
    process.exit(1)
  }
}

build()
