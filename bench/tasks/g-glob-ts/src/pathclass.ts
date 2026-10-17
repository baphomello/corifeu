// Classify a path against an ordered list of file classes.
//
// Task C, function for function and case for case, in TypeScript. With the
// Python port in `d-glob-py` this makes the same decomposition measurable in
// three languages, which is the only way a number about Corifeu can be told
// apart from a number about a compiler.
//
// The tests live in this file because every other task's do: one artefact per
// slice is what the runtime models today. `vitest.config.ts` includes this path
// explicitly, because vitest will not collect a file that is not named
// `*.test.ts` even when it is passed on the command line.

import { describe, expect, test } from "vitest";

/** A named class and the glob patterns that select files into it. */
export interface FileClass {
  name: string;
  include: string[];
}

/** Return the name of the first class whose patterns match `path`. */
export function classify(classes: FileClass[], path: string): string | null {
  for (const c of classes) {
    if (c.include.some((p) => globMatch(p, path))) {
      return c.name;
    }
  }
  return null;
}

/** Split a path into its non-empty components. */
function splitSegs(path: string): string[] {
  return path.split("/").filter((seg) => seg.length > 0);
}

/**
 * Match one pattern segment against one path component.
 * `*` matches any run of characters, `?` matches exactly one.
 */
function matchSeg(pattern: string, component: string): boolean {
  return matchSegImpl(pattern, component);
}

/** Backtracking matcher for a single pattern segment. */
function matchSegImpl(pattern: string, component: string): boolean {
  const rec = (p: number, c: number): boolean => {
    if (p === pattern.length) {
      return c === component.length;
    }
    if (pattern[p] === "*") {
      // `*` matches any run: try consuming zero, one, two, ... characters.
      for (let star = c; star <= component.length; star++) {
        if (rec(p + 1, star)) {
          return true;
        }
      }
      return false;
    }
    if (pattern[p] === "?") {
      return c < component.length && rec(p + 1, c + 1);
    }
    return c < component.length && pattern[p] === component[c] && rec(p + 1, c + 1);
  };
  return rec(0, 0);
}

/**
 * Match a whole glob pattern against a whole path.
 * `**` matches any number of path components, including none.
 */
function globMatch(pattern: string, path: string): boolean {
  const pats = splitSegs(pattern);
  const pathSegs = splitSegs(path);

  let consumed = 0;
  let pi = 0;
  while (pi < pats.length) {
    if (pats[pi] === "**") {
      // `**` can match any number of remaining path components.
      for (let take = 0; take <= pathSegs.length - consumed; take++) {
        if (globMatchSuffix(pats.slice(pi + 1), pathSegs.slice(consumed + take))) {
          return true;
        }
      }
      return false;
    }
    if (consumed < pathSegs.length && matchSeg(pats[pi], pathSegs[consumed])) {
      consumed++;
      pi++;
    } else {
      return false;
    }
  }

  return consumed === pathSegs.length;
}

function globMatchSuffix(pattern: string[], path: string[]): boolean {
  let pi = 0;
  let si = 0;
  while (pi < pattern.length && si < path.length) {
    if (pattern[pi] === "**") {
      // `**` matches any number of remaining components; try the rest of the
      // pattern against every split point.
      for (let take = 0; take <= path.length - si; take++) {
        if (globMatchSuffix(pattern.slice(pi + 1), path.slice(si + take))) {
          return true;
        }
      }
      return false;
    }
    if (matchSeg(pattern[pi], path[si])) {
      si++;
      pi++;
    } else {
      return false;
    }
  }

  return pi === pattern.length && si === path.length;
}

describe("slice1_segs", () => {
  test("splits on slash", () => {
    expect(splitSegs("src/bank/deposit.ts")).toEqual(["src", "bank", "deposit.ts"]);
  });
  test("single component", () => {
    expect(splitSegs("Cargo.toml")).toEqual(["Cargo.toml"]);
  });
  test("drops empty components", () => {
    expect(splitSegs("a//b")).toEqual(["a", "b"]);
    expect(splitSegs("/a/b/")).toEqual(["a", "b"]);
  });
  test("empty path has no components", () => {
    expect(splitSegs("")).toEqual([]);
  });
});

describe("slice2_seg", () => {
  test("literal must be exact", () => {
    expect(matchSeg("src", "src")).toBe(true);
    expect(matchSeg("src", "lib")).toBe(false);
  });
  test("case is significant", () => {
    expect(matchSeg("SRC", "src")).toBe(false);
  });
  test("star matches any run", () => {
    expect(matchSeg("*.ts", "deposit.ts")).toBe(true);
    expect(matchSeg("*.ts", "deposit.js")).toBe(false);
  });
  test("star matches nothing too", () => {
    expect(matchSeg("*.ts", ".ts")).toBe(true);
    expect(matchSeg("*", "")).toBe(true);
  });
  test("bare star matches anything", () => {
    expect(matchSeg("*", "deposit.ts")).toBe(true);
  });
  test("question matches exactly one", () => {
    expect(matchSeg("?.ts", "a.ts")).toBe(true);
    expect(matchSeg("?.ts", "ab.ts")).toBe(false);
    expect(matchSeg("?.ts", ".ts")).toBe(false);
  });
  test("star backtracks", () => {
    // A greedy `*` that never gives characters back fails this.
    expect(matchSeg("*_test.*", "deposit_test.ts")).toBe(true);
    expect(matchSeg("*_test.*", "a_test_test.ts")).toBe(true);
    expect(matchSeg("*_test.*", "deposit.ts")).toBe(false);
  });
  test("several wildcards", () => {
    expect(matchSeg("*-*.spec.?s", "user-auth.spec.ts")).toBe(true);
    expect(matchSeg("*-*.spec.?s", "userauth.spec.ts")).toBe(false);
  });
});

describe("slice3_glob", () => {
  test("literal path", () => {
    expect(globMatch("Cargo.toml", "Cargo.toml")).toBe(true);
    expect(globMatch("Cargo.toml", "Cargo.lock")).toBe(false);
  });
  test("star does not cross a slash", () => {
    expect(globMatch("*.toml", "Cargo.toml")).toBe(true);
    expect(globMatch("*.toml", "crates/Cargo.toml")).toBe(false);
  });
  test("double star absorbs components", () => {
    expect(globMatch("src/**", "src/bank/deposit.ts")).toBe(true);
    expect(globMatch("src/**", "src/a.ts")).toBe(true);
  });
  test("double star matches zero components", () => {
    expect(globMatch("**/*_test.*", "deposit_test.ts")).toBe(true);
    expect(globMatch("**/*.md", "README.md")).toBe(true);
  });
  test("double star in the middle", () => {
    expect(globMatch("**/*_test.*", "src/bank/deposit_test.ts")).toBe(true);
    expect(globMatch("docs/**/*.md", "docs/a/b/c.md")).toBe(true);
  });
  test("single star segment is exactly one", () => {
    expect(globMatch("src/*/mod.rs", "src/a/mod.rs")).toBe(true);
    expect(globMatch("src/*/mod.rs", "src/a/b/mod.rs")).toBe(false);
    expect(globMatch("src/*/mod.rs", "src/mod.rs")).toBe(false);
  });
  test("bare double star matches everything", () => {
    expect(globMatch("**", "a/b/c.ts")).toBe(true);
    expect(globMatch("**", "x")).toBe(true);
  });
  test("non matching returns false", () => {
    expect(globMatch("src/**", "docs/a.md")).toBe(false);
    expect(globMatch("tests/**", "src/lib.rs")).toBe(false);
  });
  test("pattern longer than path", () => {
    expect(globMatch("a/b/c", "a/b")).toBe(false);
  });
});

describe("slice4_classify", () => {
  const cls = (name: string, include: string[]): FileClass => ({ name, include });

  test("first match wins", () => {
    const classes = [cls("test", ["**/*_test.*"]), cls("source", ["src/**"])];
    expect(classify(classes, "src/bank/deposit_test.ts")).toBe("test");
    expect(classify(classes, "src/bank/deposit.ts")).toBe("source");
  });
  test("order decides the answer", () => {
    const classes = [cls("source", ["src/**"]), cls("test", ["**/*_test.*"])];
    expect(classify(classes, "src/bank/deposit_test.ts")).toBe("source");
  });
  test("no class matches", () => {
    expect(classify([cls("source", ["src/**"])], "README.md")).toBe(null);
  });
  test("any pattern in a class matches", () => {
    const classes = [cls("config", ["*.toml", "*.yaml", ".github/**"])];
    expect(classify(classes, "Cargo.toml")).toBe("config");
    expect(classify(classes, "app.yaml")).toBe("config");
    expect(classify(classes, ".github/workflows/ci.yml")).toBe("config");
  });
  test("empty class list matches nothing", () => {
    expect(classify([], "src/a.ts")).toBe(null);
  });
});
