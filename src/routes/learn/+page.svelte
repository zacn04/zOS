<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";

  type Problem = {
    id: string;
    topic: string;
    difficulty: number;
    statement: string;
    solution_sketch: string;
  };

  const skillTopics = [
    { id: "rl_theory", name: "RL Theory" },
    { id: "ml_theory", name: "ML Theory" },
    { id: "ai_research", name: "AI Research" },
    { id: "coding_debugging", name: "Coding Debugging" },
    { id: "algorithms", name: "Algorithms & Data Structures" },
    { id: "production_engineering", name: "Production Engineering" },
    { id: "analysis_math", name: "Analysis & Real Math" },
    { id: "putnam_competition", name: "Putnam/Competition Math" },
    { id: "proof_strategy", name: "Proof Strategy" },
    { id: "logical_reasoning", name: "Logical Reasoning" },
  ];

  let selectedTopic = $state<string | null>(null);
  let problems = $state<Problem[]>([]);
  let loading = $state(false);
  let error = $state("");

  async function loadProblems(topic: string) {
    try {
      loading = true;
      error = "";
      // Clear previous problems immediately
      problems = [];
      selectedTopic = null;
      
      const probs = await invoke<Problem[]>("get_problems_by_topic", { topic });
      
      // Validate all problems match the requested topic
      const mismatched = probs.filter(p => p.topic !== topic);
      if (mismatched.length > 0) {
        console.error(`[Learn] Found ${mismatched.length} problems with incorrect topics:`, mismatched);
        error = `Warning: ${mismatched.length} problem(s) have incorrect topics`;
      }
      
      problems = probs;
      selectedTopic = topic;
    } catch (err) {
      error = String(err);
      problems = [];
    } finally {
      loading = false;
    }
  }

  function startProblem(problem: Problem) {
    // Navigate to solve page with problem context
    // Add source=user to prevent unnecessary precomputation
    goto(`/solve?problem=${problem.id}&source=user`);
  }
</script>

<div style="padding: 24px; font-family: sans-serif; max-width: 1200px; margin: 0 auto;">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
    <h1 style="margin: 0;">Learn</h1>
    <div style="display: flex; gap: 12px;">
      <button
        on:click={() => goto("/solve")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Solve
      </button>
      <button
        on:click={() => goto("/improve")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Improve
      </button>
    </div>
  </div>

  <p style="color: #666; margin-bottom: 24px;">
    Choose a skill area to practice. Problems are organized by topic.
  </p>

  {#if error}
    <div class="error-box">
      <strong>Error:</strong> {error}
    </div>
  {/if}

  <div style="display: grid; grid-template-columns: repeat(auto-fill, minmax(250px, 1fr)); gap: 16px; margin-bottom: 32px;">
    {#each skillTopics as topic}
      <button
        on:click={() => loadProblems(topic.id)}
        style="padding: 16px; background: #fff; border: 2px solid #ddd; border-radius: 8px; cursor: pointer; text-align: left; transition: all 0.2s;"
        on:mouseenter={(e) => e.currentTarget.style.borderColor = "#396cd8"}
        on:mouseleave={(e) => e.currentTarget.style.borderColor = "#ddd"}
      >
        <h3 style="margin: 0 0 8px 0; color: #333;">{topic.name}</h3>
        <p style="margin: 0; color: #666; font-size: 14px;">Practice {topic.name.toLowerCase()}</p>
      </button>
    {/each}
  </div>

  {#if loading}
    <div style="padding: 16px; text-align: center;">Loading problems...</div>
  {/if}

  {#if selectedTopic && problems.length > 0}
    <div>
      <h2 style="margin-bottom: 16px;">
        {skillTopics.find(t => t.id === selectedTopic)?.name} Problems
      </h2>
      <div style="display: flex; flex-direction: column; gap: 16px;">
        {#each problems as problem}
          <div style="padding: 16px; background: #f5f5f5; border-radius: 4px; border-left: 4px solid #396cd8;" class="problem-card">
            <div style="display: flex; justify-content: space-between; align-items: start; margin-bottom: 8px;">
              <span style="font-weight: bold;" class="problem-id">{problem.id}</span>
              <span style="color: #666; font-size: 14px;" class="problem-difficulty">
                Difficulty: {(problem.difficulty * 100).toFixed(0)}%
              </span>
            </div>
            <p style="margin: 0 0 12px 0; line-height: 1.6; white-space: pre-wrap;" class="problem-statement-preview">
              {problem.statement.substring(0, 200)}{problem.statement.length > 200 ? "..." : ""}
            </p>
            <button
              on:click={() => startProblem(problem)}
              style="padding: 6px 12px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
            >
              Start Problem 
            </button>
          </div>
        {/each}
      </div>
    </div>
  {/if}

  {#if selectedTopic && problems.length === 0 && !loading}
    <div style="padding: 16px; text-align: center; color: #666;">
      No problems available for this topic yet.
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

  .error-box {
    margin-top: 24px;
    padding: 12px;
    background: #ffebee;
    border: 1px solid #f44336;
    border-radius: 4px;
    color: #c62828;
  }

  .problem-card {
    background: #ffffff;
    border: 1px solid #e0e0e0;
  }

  .problem-id {
    color: #212121;
  }

  .problem-difficulty {
    color: #666;
  }

  .problem-statement-preview {
    color: #212121;
  }

  @media (prefers-color-scheme: dark) {
    :global(body) {
      background-color: #1e1e1e;
      color: #e0e0e0;
    }

    .error-box {
      background: #b71c1c;
      border-color: #d32f2f;
      color: #ffcdd2;
    }

    .problem-card {
      background: #ffffff;
      border-color: #e0e0e0;
    }

    .problem-id {
      color: #212121;
    }

    .problem-difficulty {
      color: #666;
    }

    .problem-statement-preview {
      color: #212121;
    }
  }
</style>

