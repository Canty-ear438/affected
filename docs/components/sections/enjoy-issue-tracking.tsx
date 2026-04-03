"use client";

import { Features } from "../features";
import {
  AutomatedBacklogIcon,
  CustomViewsIcon,
  DiscussionIcon,
  IssuesIcon,
  ParentSubIcon,
  WorkflowsIcon,
} from "../icons/features";

export const EnjoyIssueTracking = () => {
  return (
    <Features color="0,240,255" colorDark="0,136,170">
      <Features.Main
        title={
          <>
            How it
            <br />
            works
          </>
        }
        image=""
        text="From git diff to targeted test execution — six steps, milliseconds. affected builds a dependency graph of your monorepo, diffs against your base branch, and runs commands on only the affected packages."
      />
      <Features.Grid
        features={[
          {
            icon: ParentSubIcon,
            title: "Detect.",
            text: "Scans for manifest files to identify your ecosystem.",
          },
          {
            icon: AutomatedBacklogIcon,
            title: "Resolve.",
            text: "Parses manifests into a full dependency graph.",
          },
          {
            icon: WorkflowsIcon,
            title: "Diff.",
            text: "Computes changed files via libgit2.",
          },
          {
            icon: CustomViewsIcon,
            title: "Map.",
            text: "Maps each changed file to its owning package.",
          },
          {
            icon: DiscussionIcon,
            title: "Traverse.",
            text: "Reverse BFS to find all dependents.",
          },
          {
            icon: IssuesIcon,
            title: "Execute.",
            text: "Runs your command on only affected packages.",
          },
        ]}
      />
      <Features.Cards
        features={[
          {
            image: "",
            imageClassName: "",
            title: "The --explain flag",
            text: "See exactly why each package is affected. Full dependency chain visualization from changed file to impacted package.",
          },
          {
            image: "",
            imageClassName: "",
            title: "Watch mode",
            text: "Re-runs on file change. Built-in debouncing for smooth dev loops. Your tests stay in sync as you code.",
          },
        ]}
      />
    </Features>
  );
};
