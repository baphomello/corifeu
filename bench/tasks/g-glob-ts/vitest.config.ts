// The artefact carries its own tests, as every other measurement task does.
// vitest will not collect a file that is not named `*.test.ts`, not even when
// it is passed on the command line, so the one file is named here.
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: { include: ["src/pathclass.ts"] },
});
