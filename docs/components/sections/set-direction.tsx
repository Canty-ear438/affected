"use client";

import { Features } from "../features";
import { Container } from "../container";
import { Button } from "../button";

const installMethods = [
  {
    title: "Homebrew",
    command: "brew install Rani367/tap/affected",
  },
  {
    title: "Cargo",
    command: "cargo install affected-cli",
  },
  {
    title: "uv / pipx",
    command: "uv tool install affected",
  },
  {
    title: "GitHub Actions",
    command: "uses: Rani367/setup-affected@v1",
  },
];

export const SetDirection = () => {
  return (
    <Features color="0,255,102" colorDark="0,170,68">
      <Features.Main
        title={
          <>
            Get running in
            <br />
            60 seconds
          </>
        }
        image=""
        text="Choose your preferred method. All roads lead to affected."
      />
      <Container>
        <div className="mb-16 grid w-full grid-cols-1 gap-6 md:mb-[14rem] md:grid-cols-2">
          {installMethods.map(({ title, command }) => (
            <div
              key={title}
              className="relative overflow-hidden rounded-[2.4rem] border border-transparent-white bg-[radial-gradient(ellipse_at_center,rgba(var(--feature-color),0.15),transparent)] py-6 px-8 before:pointer-events-none before:absolute before:inset-0 before:bg-glass-gradient md:rounded-[4.8rem] md:p-14"
            >
              <h3 className="mb-4 text-2xl text-white">{title}</h3>
              <div className="rounded-[0.8rem] border border-transparent-white bg-[rgba(255,255,255,0.03)] px-5 py-4 font-mono text-sm text-primary-text">
                <span className="mr-2 select-none text-grey">$</span>
                <span className="text-white">{command}</span>
              </div>
            </div>
          ))}
        </div>
        <div className="mb-16 text-center md:mb-[7.2rem]">
          <p className="mb-6 text-lg text-primary-text">
            Star on GitHub to support the project
          </p>
          <Button
            href="https://github.com/Rani367/affected"
            variant="primary"
            size="large"
          >
            Star on GitHub
            <svg
              className="ml-2"
              width="16"
              height="16"
              viewBox="0 0 16 16"
              fill="currentColor"
            >
              <path d="M8 .25a.75.75 0 01.673.418l1.882 3.815 4.21.612a.75.75 0 01.416 1.279l-3.046 2.97.719 4.192a.75.75 0 01-1.088.791L8 12.347l-3.766 1.98a.75.75 0 01-1.088-.79l.72-4.194L.818 6.374a.75.75 0 01.416-1.28l4.21-.611L7.327.668A.75.75 0 018 .25z" />
            </svg>
          </Button>
        </div>
      </Container>
    </Features>
  );
};
