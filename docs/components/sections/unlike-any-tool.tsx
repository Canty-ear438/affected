import { Container } from "../container";

export const UnlikeAnyTool = () => (
  <div className="text-white">
    <Container>
      <div className="text-center">
        <h2 className="mb-4 text-4xl md:mb-7 md:text-7xl">
          Built different.
        </h2>
        <p className="mx-auto mb-12 max-w-[68rem] text-lg text-primary-text md:mb-7 md:text-xl">
          Zero config. Zero dependencies. Just one binary that understands your
          entire monorepo.
        </p>
      </div>
    </Container>
    <div className="h-[48rem] overflow-hidden md:h-auto md:overflow-auto">
      <div className="flex snap-x snap-mandatory gap-6 overflow-x-auto px-8 pb-12 md:flex-wrap md:overflow-hidden">
        <div className="relative flex min-h-[48rem] w-full shrink-0 snap-center flex-col items-center justify-end overflow-hidden rounded-[4.8rem] border border-transparent-white bg-glass-gradient p-8 text-center md:max-w-[calc(66.66%-12px)] md:basis-[calc(66.66%-12px)] md:p-14">
          <div className="mb-8 w-full max-w-[48rem] rounded-[1.2rem] border border-transparent-white bg-[rgba(255,255,255,0.03)] p-6 text-left font-mono text-sm leading-relaxed">
            <span className="text-grey">$</span>{" "}
            <span className="text-white">affected test --base main</span>
            <br />
            <span className="text-primary-text">
              Detected ecosystem: Cargo workspace
            </span>
            <br />
            <span className="text-primary-text">
              Found 23 packages, 4 affected
            </span>
            <br />
            <span style={{ color: "#00ff66" }}>
              Running: cargo test -p core -p api -p cli -p utils
            </span>
          </div>
          <p className="mb-4 text-3xl">Zero Configuration</p>
          <p className="text-md text-primary-text">
            Auto-detects your ecosystem from manifest files. Cargo.toml? Rust
            workspace. package.json? Node monorepo. No setup required.
          </p>
        </div>
        <div className="relative flex min-h-[48rem] w-full shrink-0 snap-center flex-col items-center justify-end overflow-hidden rounded-[4.8rem] border border-transparent-white bg-glass-gradient p-8 text-center md:basis-[calc(33.33%-12px)] md:p-14">
          <div className="mb-8 flex flex-col items-center gap-4">
            <span className="text-[6rem] font-bold leading-none text-white opacity-90">
              ~5
              <span className="text-3xl text-primary-text">MB</span>
            </span>
            <div className="flex items-center gap-3 text-sm text-primary-text">
              <span className="inline-block h-2 w-2 rounded-full bg-white" />
              Single binary
              <span className="inline-block h-2 w-2 rounded-full bg-grey" />
              No runtime
            </div>
          </div>
          <p className="mb-4 text-3xl">Lightning Fast</p>
          <p className="text-md text-primary-text">
            Single ~5MB Rust binary. No Node.js, no JVM, no runtime
            dependencies. Starts in milliseconds.
          </p>
        </div>
        <div className="relative flex min-h-[48rem] w-full shrink-0 snap-center flex-col items-center justify-end overflow-hidden rounded-[4.8rem] border border-transparent-white bg-glass-gradient p-8 text-center md:basis-[calc(33.33%-12px)] md:p-14">
          <div className="mb-8 flex flex-col items-center gap-3">
            <div className="flex items-center gap-2 text-sm">
              <span
                className="rounded-[0.6rem] border border-transparent-white px-3 py-1"
                style={{ color: "#00ff66" }}
              >
                core
              </span>
              <svg
                width="16"
                height="16"
                viewBox="0 0 16 16"
                fill="#8A8F98"
              >
                <path d="M5.47 11.47a.75.75 0 001.06 1.06l4-4a.75.75 0 00.02-1.06l-4-4a.75.75 0 00-1.08 1.04L8.94 8l-3.47 3.47z" />
              </svg>
              <span className="rounded-[0.6rem] border border-transparent-white px-3 py-1 text-white">
                api
              </span>
              <svg
                width="16"
                height="16"
                viewBox="0 0 16 16"
                fill="#8A8F98"
              >
                <path d="M5.47 11.47a.75.75 0 001.06 1.06l4-4a.75.75 0 00.02-1.06l-4-4a.75.75 0 00-1.08 1.04L8.94 8l-3.47 3.47z" />
              </svg>
              <span className="rounded-[0.6rem] border border-transparent-white px-3 py-1 text-white">
                web
              </span>
            </div>
            <p className="text-xs text-primary-text">
              core changed → api + web affected
            </p>
          </div>
          <p className="mb-4 text-3xl">Full Blast Radius</p>
          <p className="text-md text-primary-text">
            Transitive dependency graph analysis. If core changes and api depends
            on it, both are affected.
          </p>
        </div>
        <div className="relative flex min-h-[48rem] w-full shrink-0 snap-center flex-col items-center justify-start overflow-hidden rounded-[4.8rem] border border-transparent-white bg-glass-gradient p-8 text-center md:max-w-[calc(66.66%-12px)] md:basis-[calc(66.66%-12px)] md:p-14">
          <div className="mb-8 w-full max-w-[48rem] rounded-[1.2rem] border border-transparent-white bg-[rgba(255,255,255,0.03)] p-6 text-left font-mono text-xs leading-relaxed">
            <span className="text-primary-text"># .github/workflows/ci.yml</span>
            <br />
            <span className="text-white">- uses:</span>{" "}
            <span style={{ color: "#00add8" }}>
              Rani367/setup-affected@v1
            </span>
            <br />
            <span className="text-white">- run:</span>{" "}
            <span style={{ color: "#f9ad00" }}>
              affected test --base ${"${{ github.event.pull_request.base.sha }}"}
            </span>
            <br />
            <span className="text-white">- run:</span>{" "}
            <span style={{ color: "#f9ad00" }}>
              affected lint --output junit
            </span>
          </div>
          <p className="mb-4 text-3xl">CI-Native</p>
          <p className="text-md text-primary-text">
            GitHub Actions, GitLab CI, CircleCI, Azure Pipelines. Dynamic
            matrices, PR comment bot, JUnit output.
          </p>
        </div>
      </div>
    </div>
  </div>
);
