<script>
  import { onMount, onDestroy } from "svelte";
  import { invoke } from "../tauri/core-proxy.js";
  import { t } from "svelte-i18n";
  import { currentFile, isPlaying, isPaused, currentPosition } from "../stores/player.js";

  let canvas;
  let ctx;
  let notes = [];
  let animationFrame;
  let displayWidth = 0;
  let displayHeight = 0;

  // Visual settings
  const KEY_COUNT = 21;
  const VISIBLE_SECONDS = 3;
  const NOTE_COLORS = {
    low: "#22c55e",
    mid: "#3b82f6",
    high: "#a855f7",
  };

  function getOctaveColor(keyIndex) {
    if (keyIndex < 7) return NOTE_COLORS.low;
    if (keyIndex < 14) return NOTE_COLORS.mid;
    return NOTE_COLORS.high;
  }

  async function loadNotes() {
    if (!$currentFile) {
      notes = [];
      return;
    }
    try {
      notes = await invoke("get_visualizer_notes");
    } catch (e) {
      notes = [];
    }
  }

  function draw() {
    if (!ctx || !canvas || !displayWidth || !displayHeight) {
      if ($isPlaying && !$isPaused) {
        animationFrame = requestAnimationFrame(draw);
      }
      return;
    }

    const width = displayWidth;
    const height = displayHeight;
    const keyWidth = width / KEY_COUNT;

    // Use backend position directly
    const currentTimeMs = $currentPosition * 1000;
    const visibleMs = VISIBLE_SECONDS * 1000;

    // Clear
    ctx.fillStyle = "#0a0a0a";
    ctx.fillRect(0, 0, width, height);

    // Grid
    ctx.strokeStyle = "rgba(255, 255, 255, 0.03)";
    ctx.lineWidth = 1;
    for (let i = 1; i < KEY_COUNT; i++) {
      ctx.beginPath();
      ctx.moveTo(i * keyWidth, 0);
      ctx.lineTo(i * keyWidth, height);
      ctx.stroke();
    }

    ctx.strokeStyle = "rgba(255, 255, 255, 0.1)";
    [7, 14].forEach(i => {
      ctx.beginPath();
      ctx.moveTo(i * keyWidth, 0);
      ctx.lineTo(i * keyWidth, height);
      ctx.stroke();
    });

    // Draw notes
    for (const note of notes) {
      const noteStartMs = note.time_ms;
      const noteEndMs = note.time_ms + note.duration_ms;

      if (noteEndMs < currentTimeMs - 100) continue;
      if (noteStartMs > currentTimeMs + visibleMs) continue;

      const yStart = height - ((noteStartMs - currentTimeMs) / visibleMs) * height;
      const yEnd = height - ((noteEndMs - currentTimeMs) / visibleMs) * height;
      const noteHeight = Math.max(yStart - yEnd, 4);

      const x = note.key_index * keyWidth + 1;
      const noteWidth = keyWidth - 2;
      const color = getOctaveColor(note.key_index);
      const isActive = yStart >= height - 20 && yStart <= height + 50;

      let alpha = 0.5;
      if (yEnd < 20) alpha = Math.max(0, (yEnd + noteHeight) / 40) * 0.5;
      if (isActive) alpha = 1;

      const radius = 2;
      const drawY = Math.max(-noteHeight, yEnd);

      if (isActive) {
        ctx.shadowColor = color;
        ctx.shadowBlur = 20;
        ctx.globalAlpha = 1;
        ctx.fillStyle = "#ffffff";
        ctx.beginPath();
        ctx.roundRect(x, drawY, noteWidth, noteHeight, radius);
        ctx.fill();

        ctx.shadowBlur = 25;
        ctx.globalAlpha = 0.9;
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.roundRect(x - 2, drawY - 2, noteWidth + 4, noteHeight + 4, radius + 1);
        ctx.fill();
        ctx.shadowBlur = 0;

        ctx.globalAlpha = 0.6;
        ctx.fillRect(x - 4, height - 4, noteWidth + 8, 4);
      } else {
        ctx.globalAlpha = alpha;
        ctx.fillStyle = color;
        ctx.beginPath();
        ctx.roundRect(x, drawY, noteWidth, noteHeight, radius);
        ctx.fill();
      }
    }

    ctx.globalAlpha = 1;

    // Top fade
    const fadeGradient = ctx.createLinearGradient(0, 0, 0, 30);
    fadeGradient.addColorStop(0, "#0a0a0a");
    fadeGradient.addColorStop(1, "transparent");
    ctx.fillStyle = fadeGradient;
    ctx.fillRect(0, 0, width, 30);

    // Playhead
    ctx.strokeStyle = "rgba(255, 255, 255, 0.4)";
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(0, height - 1);
    ctx.lineTo(width, height - 1);
    ctx.stroke();

    if ($isPlaying && !$isPaused) {
      animationFrame = requestAnimationFrame(draw);
    }
  }

  function resizeCanvas() {
    if (!canvas) return;
    const rect = canvas.parentElement.getBoundingClientRect();
    const dpr = window.devicePixelRatio || 1;
    displayWidth = rect.width;
    displayHeight = rect.height;
    canvas.width = displayWidth * dpr;
    canvas.height = displayHeight * dpr;
    canvas.style.width = displayWidth + "px";
    canvas.style.height = displayHeight + "px";
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    draw();
  }

  onMount(() => {
    ctx = canvas.getContext("2d");
    resizeCanvas();
    window.addEventListener("resize", resizeCanvas);
  });

  onDestroy(() => {
    if (animationFrame) cancelAnimationFrame(animationFrame);
    window.removeEventListener("resize", resizeCanvas);
  });

  $: if ($currentFile) loadNotes();
  $: if ($isPlaying && !$isPaused) {
    if (!animationFrame) animationFrame = requestAnimationFrame(draw);
  } else {
    if (animationFrame) { cancelAnimationFrame(animationFrame); animationFrame = null; }
    draw();
  }
</script>

<div class="w-full h-full bg-[#0a0a0a] overflow-hidden relative">
  <canvas bind:this={canvas} class="w-full h-full"></canvas>
  {#if !$currentFile}
    <div class="absolute inset-0 flex items-center justify-center text-white/20 text-xs">
      {$t("trackSelector.selectTrack")}
    </div>
  {/if}
</div>
