<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { onMount } from "svelte";

  type SessionRecord = {
    session_id: string;
    problem_id: string;
    skill: string;
    user_attempt: string;
    issues: string[];
    eval_summary: string;
    skill_before: number;
    skill_after: number;
    difficulty: number;
    timestamp: number;
  };

  type AnalyticsPayload = {
    skill_history: Record<string, Array<[number, number]>>;
    session_counts: Record<string, number>;
    avg_difficulty: Record<string, number>;
    weekly_trends: Record<string, number>;
  };

  const skillNames: Record<string, string> = {
    rl_theory: "RL Theory",
    ml_theory: "ML Theory",
    ai_research: "AI Research",
    coding_debugging: "Coding Debugging",
    algorithms: "Algorithms",
    production_engineering: "Production Engineering",
    analysis_math: "Analysis & Math",
    putnam_competition: "Putnam/Competition",
    proof_strategy: "Proof Strategy",
    logical_reasoning: "Logical Reasoning",
  };

  let sessions = $state<SessionRecord[]>([]);
  let analytics = $state<AnalyticsPayload | null>(null);
  let loading = $state(true);
  let error = $state("");
  let showAnalytics = $state(true);

  function fmt(timestamp: number): string {
    return new Date(timestamp * 1000).toLocaleString();
  }

  function getSkillDelta(before: number, after: number): string {
    const delta = after - before;
    const sign = delta >= 0 ? "+" : "";
    return `${sign}${delta.toFixed(3)}`;
  }

  function getSkillDeltaColor(before: number, after: number): string {
    const delta = after - before;
    if (delta > 0) return "#4caf50";
    if (delta < 0) return "#f44336";
    return "#9e9e9e";
  }

  onMount(async () => {
    try {
      loading = true;
      error = "";
      [sessions, analytics] = await Promise.all([
        invoke<SessionRecord[]>("get_session_history"),
        invoke<AnalyticsPayload>("get_analytics_data").catch(() => null)
      ]);
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  });

  function getTrendArrow(trend: number): string {
    if (trend > 0.01) return "↑";
    if (trend < -0.01) return "↓";
    return "→";
  }

  function getTrendColor(trend: number): string {
    if (trend > 0.01) return "#4caf50";
    if (trend < -0.01) return "#f44336";
    return "#9e9e9e";
  }

  function renderSkillProgressionChart(skill: string, history: Array<[number, number]>): string {
    if (history.length === 0) return "";
    
    const width = 400;
    const height = 150;
    const padding = 40;
    const chartWidth = width - padding * 2;
    const chartHeight = height - padding * 2;
    
    const values = history.map(([_, val]) => val);
    const minVal = Math.min(...values, 0);
    const maxVal = Math.max(...values, 1);
    const range = maxVal - minVal || 1;
    
    const points = history.map(([ts, val], i) => {
      const x = padding + (i / Math.max(history.length - 1, 1)) * chartWidth;
      const y = padding + chartHeight - ((val - minVal) / range) * chartHeight;
      return `${x},${y}`;
    }).join(" ");
    
    return `<svg width="${width}" height="${height}" style="background: #f9f9f9; border-radius: 4px;">
      <polyline points="${points}" fill="none" stroke="#396cd8" stroke-width="2"/>
      ${history.map(([ts, val], i) => {
        const x = padding + (i / Math.max(history.length - 1, 1)) * chartWidth;
        const y = padding + chartHeight - ((val - minVal) / range) * chartHeight;
        return `<circle cx="${x}" cy="${y}" r="3" fill="#396cd8"/>`;
      }).join("")}
    </svg>`;
  }

  function renderBarChart(data: Record<string, number>, maxValue: number, color: string): string {
    const width = 400;
    const height = 200;
    const barWidth = 30;
    const spacing = 10;
    const chartHeight = height - 40;
    
    const entries = Object.entries(data);
    const totalWidth = entries.length * (barWidth + spacing);
    const startX = (width - totalWidth) / 2;
    
    const bars = entries.map(([skill, value], i) => {
      const x = startX + i * (barWidth + spacing);
      const barHeight = (value / maxValue) * chartHeight;
      const y = height - barHeight - 20;
      return `<rect x="${x}" y="${y}" width="${barWidth}" height="${barHeight}" fill="${color}"/>
        <text x="${x + barWidth/2}" y="${height - 5}" text-anchor="middle" font-size="10" fill="#212121">${skill.substring(0, 8)}</text>
        <text x="${x + barWidth/2}" y="${y - 5}" text-anchor="middle" font-size="10" font-weight="bold" fill="#212121">${value.toFixed(1)}</text>`;
    }).join("");
    
    return `<svg width="${width}" height="${height}" style="background: #f9f9f9; border-radius: 4px;">${bars}</svg>`;
  }
</script>

<div style="padding: 24px; font-family: sans-serif; max-width: 1200px; margin: 0 auto;">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
    <h1 style="margin: 0;">History</h1>
    <div style="display: flex; gap: 12px;">
      <button
        on:click={() => goto("/solve")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Solve
      </button>
      <button
        on:click={() => goto("/learn")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Learn
      </button>
      <button
        on:click={() => goto("/improve")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Improve
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-box">
      <strong>Error:</strong> {error}
    </div>
  {/if}

  {#if loading}
    <div style="padding: 16px; text-align: center;">Loading history...</div>
  {/if}

  {#if !loading && analytics && showAnalytics}
    <div class="analytics-section">
      <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 16px;">
        <h2 style="margin: 0;">Analytics Dashboard</h2>
        <button
          on:click={() => showAnalytics = false}
          style="padding: 6px 12px; background: #757575; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 12px;"
        >
          Hide
        </button>
      </div>

      <div class="analytics-grid">
        <!-- Skill Progression Charts -->
        <div class="analytics-card">
          <h3>Skill Progression</h3>
          <div style="display: flex; flex-direction: column; gap: 16px;">
            {#each Object.entries(analytics.skill_history) as [skill, history]}
              {#if history.length > 0}
                <div>
                  <div style="display: flex; justify-content: space-between; margin-bottom: 8px;">
                    <strong>{skillNames[skill] || skill}</strong>
                    <span style="color: {getTrendColor(analytics.weekly_trends[skill] || 0)}; font-weight: bold;">
                      {getTrendArrow(analytics.weekly_trends[skill] || 0)} {(analytics.weekly_trends[skill] || 0).toFixed(3)}
                    </span>
                  </div>
                  {@html renderSkillProgressionChart(skill, history)}
                </div>
              {/if}
            {/each}
          </div>
        </div>

        <!-- Session Counts -->
        <div class="analytics-card">
          <h3>Session Counts by Skill</h3>
          {#if Object.keys(analytics.session_counts).length > 0}
            {@html renderBarChart(analytics.session_counts, Math.max(1, ...Object.values(analytics.session_counts)), "#396cd8")}
          {:else}
            <p>No session data yet</p>
          {/if}
        </div>

        <!-- Average Difficulty -->
        <div class="analytics-card">
          <h3>Average Difficulty Attempted</h3>
          {#if Object.keys(analytics.avg_difficulty).length > 0}
            {@html renderBarChart(analytics.avg_difficulty, 1.0, "#ff9800")}
          {:else}
            <p>No difficulty data yet</p>
          {/if}
        </div>

        <!-- Weekly Trends -->
        <div class="analytics-card">
          <h3>Weekly Trends</h3>
          <div style="display: flex; flex-direction: column; gap: 8px;">
            {#each Object.entries(analytics.weekly_trends) as [skill, trend]}
              <div style="display: flex; justify-content: space-between; align-items: center; padding: 8px; background: #f5f5f5; border-radius: 4px;">
                <span>{skillNames[skill] || skill}</span>
                <span style="color: {getTrendColor(trend)}; font-weight: bold; font-size: 18px;">
                  {getTrendArrow(trend)} {Math.abs(trend).toFixed(3)}
                </span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  {/if}

  {#if !loading && !showAnalytics}
    <div style="margin-bottom: 16px;">
      <button
        on:click={() => showAnalytics = true}
        style="padding: 8px 16px; background: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Show Analytics
      </button>
    </div>
  {/if}

  {#if !loading && sessions.length === 0}
    <div style="padding: 16px; text-align: center; color: #666;">
      No session history yet. Start solving problems to see your learning progress!
    </div>
  {/if}

  {#if !loading && sessions.length > 0}
    <div style="display: flex; flex-direction: column; gap: 16px;">
      {#each sessions as session}
        <div class="session-card">
          <div style="display: flex; justify-content: space-between; align-items: start; margin-bottom: 12px;">
            <div>
              <h3 style="margin: 0 0 4px 0;">
                {session.problem_id} — <span style="text-transform: capitalize;">{session.skill.replace(/_/g, " ")}</span>
              </h3>
              <p style="margin: 0; color: #666; font-size: 14px;">
                {fmt(session.timestamp)}
              </p>
            </div>
            <div style="text-align: right;">
              <div style="font-size: 18px; font-weight: bold; color: {getSkillDeltaColor(session.skill_before, session.skill_after)};">
                {getSkillDelta(session.skill_before, session.skill_after)}
              </div>
              <div style="font-size: 12px; color: #666;">
                {session.skill_before.toFixed(2)} → {session.skill_after.toFixed(2)}
              </div>
            </div>
          </div>

          {#if session.issues.length > 0}
            <div style="margin-bottom: 8px;">
              <strong style="color: #d32f2f;">Issues:</strong>
              <ul style="margin: 4px 0; padding-left: 20px;">
                {#each session.issues as issue}
                  <li style="margin-bottom: 4px; line-height: 1.4;">{issue}</li>
                {/each}
              </ul>
            </div>
          {/if}

          <div style="margin-bottom: 8px;">
            <strong>Evaluation:</strong> {session.eval_summary}
          </div>

          <details style="margin-top: 8px;">
            <summary style="cursor: pointer; color: #396cd8; font-weight: 500;">View Attempt</summary>
            <pre style="margin-top: 8px; padding: 12px; background: #f5f5f5; border-radius: 4px; white-space: pre-wrap; font-size: 12px; max-height: 200px; overflow-y: auto;">{session.user_attempt}</pre>
          </details>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    font-family: sans-serif;
    background-color: #f6f6f6;
  }

  .session-card {
    padding: 16px;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 8px;
    border-left: 4px solid #396cd8;
  }

  .error-box {
    margin-top: 24px;
    padding: 12px;
    background: #ffebee;
    border: 1px solid #f44336;
    border-radius: 4px;
    color: #c62828;
  }

  .analytics-section {
    margin-bottom: 32px;
    padding: 20px;
    background: #fff;
    border-radius: 8px;
    border: 1px solid #ddd;
  }

  .analytics-grid {
    display: grid;
    grid-template-columns: repeat(auto-fit, minmax(400px, 1fr));
    gap: 20px;
  }

  .analytics-card {
    padding: 16px;
    background: #fafafa;
    border-radius: 8px;
    border: 1px solid #e0e0e0;
  }

  .analytics-card h3 {
    margin: 0 0 16px 0;
    font-size: 18px;
    color: #333;
  }

  @media (prefers-color-scheme: dark) {
    :global(body) {
      background-color: #1e1e1e;
      color: #e0e0e0;
    }

    .session-card {
      background: #2d2d2d;
      border-color: #444;
      border-left-color: #3f51b5;
    }

    .error-box {
      background: #b71c1c;
      border-color: #d32f2f;
      color: #ffcdd2;
    }

    h1, h3 {
      color: #ffffff !important;
    }

    .analytics-section {
      background: #2d2d2d;
      border-color: #444;
    }

    .analytics-card {
      background: #1e1e1e;
      border-color: #444;
    }

    .analytics-card h3 {
      color: #e0e0e0;
    }
  }
</style>

