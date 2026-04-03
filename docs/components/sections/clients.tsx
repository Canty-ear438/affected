const ecosystems = [
  { name: "Cargo", color: "#dea584" },
  { name: "npm", color: "#cb3837" },
  { name: "pnpm", color: "#f9ad00" },
  { name: "Yarn", color: "#2c8ebb" },
  { name: "Bun", color: "#fbf0df" },
  { name: "Go", color: "#00add8" },
  { name: "Python", color: "#3776ab" },
  { name: "Maven", color: "#e76f00" },
  { name: "Gradle", color: "#1ba8a1" },
  { name: ".NET", color: "#512bd4" },
  { name: "Swift", color: "#f05138" },
  { name: "Dart", color: "#0175c2" },
  { name: "Elixir", color: "#6e4a7e" },
];

export const Clients = () => (
  <>
    <p className="mb-12 text-center text-lg text-white md:text-xl">
      <span className="text-primary-text">
        13 ecosystems, zero configuration.
      </span>
      <br className="hidden md:block" /> Auto-detected from your manifest files.
    </p>

    <div className="flex flex-wrap justify-center gap-x-6 gap-y-8">
      {ecosystems.map(({ name, color }) => (
        <span
          key={name}
          className="inline-flex items-center rounded-full border px-5 py-2 text-sm font-medium"
          style={{
            color,
            borderColor: `${color}33`,
            backgroundColor: `${color}0a`,
          }}
        >
          {name}
        </span>
      ))}
    </div>
  </>
);
