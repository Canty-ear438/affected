import {
  siRust,
  siNpm,
  siPnpm,
  siYarn,
  siBun,
  siGo,
  siPython,
  siApachemaven,
  siGradle,
  siDotnet,
  siSwift,
  siDart,
  siElixir,
} from "simple-icons";
import type { SimpleIcon } from "simple-icons";

const ecosystems: { name: string; icon: SimpleIcon; color: string }[] = [
  { name: "Cargo", icon: siRust, color: "#dea584" },
  { name: "npm", icon: siNpm, color: "#cb3837" },
  { name: "pnpm", icon: siPnpm, color: "#f9ad00" },
  { name: "Yarn", icon: siYarn, color: "#2c8ebb" },
  { name: "Bun", icon: siBun, color: "#f4d68f" },
  { name: "Go", icon: siGo, color: "#00add8" },
  { name: "Python", icon: siPython, color: "#3776ab" },
  { name: "Maven", icon: siApachemaven, color: "#e76f00" },
  { name: "Gradle", icon: siGradle, color: "#1ba8a1" },
  { name: ".NET", icon: siDotnet, color: "#a074e8" },
  { name: "Swift", icon: siSwift, color: "#f05138" },
  { name: "Dart", icon: siDart, color: "#0175c2" },
  { name: "Elixir", icon: siElixir, color: "#9b6bce" },
];

export const Clients = () => (
  <>
    <p className="mb-12 text-center text-lg text-white md:text-xl">
      <span className="text-primary-text">
        13 ecosystems, zero configuration.
      </span>
      <br className="hidden md:block" /> Auto-detected from your manifest files.
    </p>

    <div className="flex flex-wrap justify-center gap-x-3 gap-y-4">
      {ecosystems.map(({ name, icon, color }) => (
        <span
          key={name}
          className="inline-flex items-center gap-[7px] rounded-full border px-4 py-[7px] text-sm font-medium"
          style={{
            color,
            borderColor: `${color}33`,
            backgroundColor: `${color}0d`,
          }}
        >
          <svg
            width="13"
            height="13"
            viewBox="0 0 24 24"
            fill={color}
            aria-hidden
            style={{ flexShrink: 0 }}
          >
            <path d={icon.path} />
          </svg>
          {name}
        </span>
      ))}
    </div>
  </>
);
