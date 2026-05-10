<script lang="ts">
  // Smart icon renderer. Accepts either a Phosphor icon name (semantic)
  // or a single-glyph emoji/string (legacy fallback for user-defined
  // command icons that we don't want to constrain to a fixed library).
  //
  // Phosphor icon names use PascalCase matching the components in
  // `phosphor-svelte/lib/<Name>`. Below we register only the icons the
  // app actually uses — adding a new one is one line in the registry,
  // and tree-shaking keeps the bundle to just the icons listed.

  import type { Component } from "svelte";
  import Folder from "phosphor-svelte/lib/Folder";
  import FolderOpen from "phosphor-svelte/lib/FolderOpen";
  import Plus from "phosphor-svelte/lib/Plus";
  import X from "phosphor-svelte/lib/X";
  import GitBranch from "phosphor-svelte/lib/GitBranch";
  import GitDiff from "phosphor-svelte/lib/GitDiff";
  import GitPullRequest from "phosphor-svelte/lib/GitPullRequest";
  import GitCommit from "phosphor-svelte/lib/GitCommit";
  import Toolbox from "phosphor-svelte/lib/Toolbox";
  import Wrench from "phosphor-svelte/lib/Wrench";
  import Hammer from "phosphor-svelte/lib/Hammer";
  import FlaskFill from "phosphor-svelte/lib/Flask";
  import MagnifyingGlass from "phosphor-svelte/lib/MagnifyingGlass";
  import ChatCircleText from "phosphor-svelte/lib/ChatCircleText";
  import Plant from "phosphor-svelte/lib/Plant";
  import Robot from "phosphor-svelte/lib/Robot";
  import MapTrifold from "phosphor-svelte/lib/MapTrifold";
  import PaperPlaneTilt from "phosphor-svelte/lib/PaperPlaneTilt";
  import AirplaneTilt from "phosphor-svelte/lib/AirplaneTilt";
  import Sun from "phosphor-svelte/lib/Sun";
  import Moon from "phosphor-svelte/lib/Moon";
  import Bell from "phosphor-svelte/lib/Bell";
  import ArrowsClockwise from "phosphor-svelte/lib/ArrowsClockwise";
  import Check from "phosphor-svelte/lib/Check";
  import Lightning from "phosphor-svelte/lib/Lightning";
  import CaretRight from "phosphor-svelte/lib/CaretRight";
  import DotsThreeVertical from "phosphor-svelte/lib/DotsThreeVertical";
  import Terminal from "phosphor-svelte/lib/Terminal";
  import LinkSimple from "phosphor-svelte/lib/LinkSimple";
  import FilePlus from "phosphor-svelte/lib/FilePlus";
  import Trash from "phosphor-svelte/lib/Trash";
  import Tag from "phosphor-svelte/lib/Tag";

  const REGISTRY: Record<string, Component<any>> = {
    folder: Folder,
    folderOpen: FolderOpen,
    plus: Plus,
    x: X,
    close: X,
    gitBranch: GitBranch,
    gitDiff: GitDiff,
    gitPullRequest: GitPullRequest,
    gitCommit: GitCommit,
    toolbox: Toolbox,
    wrench: Wrench,
    hammer: Hammer,
    flask: FlaskFill,
    magnifyingGlass: MagnifyingGlass,
    chatCircleText: ChatCircleText,
    plant: Plant,
    robot: Robot,
    mapTrifold: MapTrifold,
    paperPlaneTilt: PaperPlaneTilt,
    airplaneTilt: AirplaneTilt,
    sun: Sun,
    moon: Moon,
    bell: Bell,
    arrowsClockwise: ArrowsClockwise,
    check: Check,
    lightning: Lightning,
    caretRight: CaretRight,
    dotsThreeVertical: DotsThreeVertical,
    terminal: Terminal,
    linkSimple: LinkSimple,
    filePlus: FilePlus,
    trash: Trash,
    tag: Tag,
  };

  type Props = {
    name: string;
    size?: number | string;
    weight?: "thin" | "light" | "regular" | "bold" | "fill" | "duotone";
    color?: string;
  };

  let { name, size = 16, weight = "regular", color = "currentColor" }: Props =
    $props();

  let resolved = $derived(REGISTRY[name]);
</script>

{#if resolved}
  {@const C = resolved}
  <C {size} {weight} {color} />
{:else}
  <!-- Fallback: render the raw string as text. Lets user-defined emoji
       icons keep working without a Phosphor mapping. -->
  <span class="emoji" style="font-size: {typeof size === 'number' ? size + 'px' : size}">
    {name}
  </span>
{/if}

<style>
  .emoji {
    line-height: 1;
    display: inline-flex;
    align-items: center;
    justify-content: center;
  }
</style>
