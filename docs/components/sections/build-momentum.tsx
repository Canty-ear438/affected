"use client";

import { Features } from "../features";
import { Container } from "../container";

const comparisonData = [
  {
    feature: "Zero config",
    affected: { value: "\u2713", highlight: true },
    nx: { value: "\u2717", highlight: false },
    turborepo: { value: "\u2717", highlight: false },
    bazel: { value: "\u2717", highlight: false },
  },
  {
    feature: "Standalone binary",
    affected: { value: "\u2713", highlight: true },
    nx: { value: "Node.js", highlight: false },
    turborepo: { value: "Node.js", highlight: false },
    bazel: { value: "JVM", highlight: false },
  },
  {
    feature: "Setup time",
    affected: { value: "1 min", highlight: true },
    nx: { value: "Hours", highlight: false },
    turborepo: { value: "Hours", highlight: false },
    bazel: { value: "Days", highlight: false },
  },
  {
    feature: "Binary size",
    affected: { value: "~5 MB", highlight: true },
    nx: { value: "~200 MB+", highlight: false },
    turborepo: { value: "~100 MB+", highlight: false },
    bazel: { value: "~500 MB+", highlight: false },
  },
  {
    feature: "Ecosystems",
    affected: { value: "13", highlight: true },
    nx: { value: "JS/TS", highlight: false },
    turborepo: { value: "JS/TS", highlight: false },
    bazel: { value: "Any", highlight: false },
  },
  {
    feature: "--explain",
    affected: { value: "\u2713", highlight: true },
    nx: { value: "\u2717", highlight: false },
    turborepo: { value: "\u2717", highlight: false },
    bazel: { value: "\u2717", highlight: false },
  },
  {
    feature: "Watch mode",
    affected: { value: "\u2713", highlight: true },
    nx: { value: "\u2713", highlight: false },
    turborepo: { value: "\u2717", highlight: false },
    bazel: { value: "\u2717", highlight: false },
  },
  {
    feature: "Multi-CI",
    affected: { value: "5 platforms", highlight: true },
    nx: { value: "GitHub", highlight: false },
    turborepo: { value: "GitHub", highlight: false },
    bazel: { value: "Custom", highlight: false },
  },
];

function CellValue({ value, highlight }: { value: string; highlight: boolean }) {
  if (value === "\u2713") {
    return (
      <span style={{ color: "#00ff66" }} className={highlight ? "font-medium" : ""}>
        {value}
      </span>
    );
  }
  if (value === "\u2717") {
    return (
      <span style={{ color: "#ff0055", opacity: 0.7 }}>{value}</span>
    );
  }
  return (
    <span className={highlight ? "text-white" : "text-primary-text"}>
      {value}
    </span>
  );
}

export const BuildMomentum = () => {
  return (
    <Features color="255,0,85" colorDark="170,0,51">
      <Features.Main
        title={
          <>
            How affected
            <br />
            stacks up
          </>
        }
        image=""
        text="The power of a build system. The simplicity of a CLI."
      />
      <Container>
        <div className="mb-16 w-full overflow-x-auto md:mb-[14rem]">
          <div className="min-w-[60rem] rounded-[2.4rem] border border-transparent-white bg-[rgba(255,255,255,0.03)]">
            {/* Header */}
            <div className="grid grid-cols-5 border-b border-transparent-white px-8 py-5 text-sm font-medium text-white">
              <div className="text-primary-text">Feature</div>
              <div style={{ color: "rgb(255,0,85)" }}>affected</div>
              <div className="text-primary-text">Nx</div>
              <div className="text-primary-text">Turborepo</div>
              <div className="text-primary-text">Bazel</div>
            </div>
            {/* Rows */}
            {comparisonData.map((row) => (
              <div
                key={row.feature}
                className="grid grid-cols-5 border-b border-transparent-white px-8 py-4 text-sm last:border-b-0"
              >
                <div className="text-white">{row.feature}</div>
                <div>
                  <CellValue {...row.affected} />
                </div>
                <div>
                  <CellValue {...row.nx} />
                </div>
                <div>
                  <CellValue {...row.turborepo} />
                </div>
                <div>
                  <CellValue {...row.bazel} />
                </div>
              </div>
            ))}
          </div>
        </div>
      </Container>
    </Features>
  );
};
