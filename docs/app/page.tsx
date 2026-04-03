import { Container } from "../components/container";
import { StarsIllustration } from "../components/icons/stars";
import { HomepageHero } from "../components/sections/homepage-hero";
import { Clients } from "../components/sections/clients";
import { UnlikeAnyTool } from "../components/sections/unlike-any-tool";
import { EnjoyIssueTracking } from "../components/sections/enjoy-issue-tracking";
import { BuildMomentum } from "../components/sections/build-momentum";
import { SetDirection } from "../components/sections/set-direction";

export default function Homepage() {
  return (
    <>
      <div className="overflow-hidden pb-[16.4rem] md:pb-[25.6rem]">
        <Container className="pt-[6.4rem]">
          <HomepageHero />
        </Container>
      </div>
      <div id="ecosystems">
        <Container>
          <Clients />
        </Container>
      </div>
      <div
        className="mask-radial-faded pointer-events-none relative z-[-1] my-[-12.8rem] h-[60rem] overflow-hidden [--color:#ffffff] before:absolute before:inset-0 before:bg-radial-faded before:opacity-[0.4] after:absolute after:top-1/2 after:-left-1/2 after:h-[142.8%] after:w-[200%] after:rounded-[50%] after:border-t after:border-[rgba(255,_255,_255,_0.08)] after:bg-background"
      >
        <StarsIllustration />
      </div>
      <div id="features"><UnlikeAnyTool /></div>
      <div id="how-it-works"><EnjoyIssueTracking /></div>
      <div id="compare"><BuildMomentum /></div>
      <div id="install"><SetDirection /></div>
    </>
  );
}
