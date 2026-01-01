<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";

  type SkillVector = {
    skills: Record<string, number>;
  };

  type Problem = {
    id: string;
    topic: string;
    difficulty: number;
    statement: string;
    solution_sketch: string;
  };

  type TaskDirective = 
    | { Adaptive: { skill: string; difficulty: number } }
    | { Review: { skill: string } };

  type CurriculumPlan = {
    tasks: TaskDirective[];
    generated_at: number;
    expires_at: number;
  };

  const skillNames: Record<string, string> = {
    rl_theory: "RL Theory",
    ml_theory: "ML Theory",
    ai_research: "AI Research",
    coding_debugging: "Coding Debugging",
    algorithms: "Algorithms & Data Structures",
    production_engineering: "Production Engineering",
    analysis_math: "Analysis & Real Math",
    putnam_competition: "Putnam/Competition Math",
    proof_strategy: "Proof Strategy",
    logical_reasoning: "Logical Reasoning",
  };

  let skills = $state<SkillVector | null>(null);
  let recommendedProblem = $state<Problem | null>(null);
  let plan = $state<CurriculumPlan | null>(null);
  let loading = $state(false);
  let error = $state("");

  async function loadSkills() {
    try {
      loading = true;
      error = "";
      const skillData = await invoke<SkillVector>("get_skills");
      skills = skillData;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function getTargetedProblem() {
    try {
      loading = true;
      error = "";
      const problem = await invoke<Problem>("get_recommended_problem");
      recommendedProblem = problem;
      
      // Propagate problem ID to URL
      goto(`/improve?problem=${problem.id}`, { replaceState: true });
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  // Function to load problem by ID
  async function loadProblemById(problemId: string) {
    if (loading) return; // Prevent concurrent loads
    loading = true;
    error = "";
    try {
      const problem = await invoke<Problem>("get_problem_by_id", { problemId: problemId });
      recommendedProblem = problem;
      
      // Propagate problem ID to URL
      goto(`/improve?problem=${problem.id}`, { replaceState: true });
      
      loading = false;
    } catch (err) {
      error = String(err);
      loading = false;
      // Clear the problem on error
      recommendedProblem = null;
    }
  }

  function getSkillColor(value: number): string {
    if (value >= 0.7) return "#4caf50";
    if (value >= 0.5) return "#ff9800";
    return "#f44336";
  }

  function getWeakestSkill(): [string, number] | null {
    if (!skills) return null;
    const entries = Object.entries(skills.skills);
    if (entries.length === 0) return null;
    return entries.reduce((min, [key, val]) => 
      val < min[1] ? [key, val] : min
    );
  }

  async function loadPlan() {
    try {
      const planData = await invoke<CurriculumPlan>("get_daily_plan");
      plan = planData;
    } catch (err) {
      // Plan might not exist yet, that's okay
      plan = null;
    }
  }

  async function refreshPlan() {
    try {
      loading = true;
      error = "";
      await invoke("refresh_daily_plan");
      await loadPlan();
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }


  function formatTask(task: TaskDirective): string {
    if ("Adaptive" in task) {
      const { skill, difficulty } = task.Adaptive;
      const diffStr = difficulty < 0.3 ? "easy" : difficulty < 0.6 ? "medium" : "hard";
      return `Practice ${skillNames[skill] || skill} (${diffStr})`;
    } else if ("Review" in task) {
      return `Review ${skillNames[task.Review.skill] || task.Review.skill}`;
    }
    return "Unknown task";
  }

  function isPlanExpired(): boolean {
    if (!plan) return true;
    return Date.now() / 1000 > plan.expires_at;
  }

  // Watch for URL changes reactively
  $effect(() => {
    const problemId = $page.url.searchParams.get("problem");
    if (problemId && problemId !== recommendedProblem?.id && !loading) {
      loadProblemById(problemId);
    }
  });

  loadSkills();
  loadPlan();
</script>

<div style="padding: 24px; font-family: sans-serif; max-width: 1200px; margin: 0 auto;">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
    <h1 style="margin: 0;">Improve</h1>
    <div style="display: flex; gap: 12px;">
      <button
        onclick={() => goto("/solve")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Solve
      </button>
      <button
        onclick={() => goto("/learn")}
        style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer;"
      >
        Learn
      </button>
    </div>
  </div>

  {#if error}
    <div class="error-box">
      <strong>Error:</strong> {error}
    </div>
  {/if}

  {#if loading && !skills}
    <div style="padding: 16px; text-align: center;">Loading skills...</div>
  {/if}

  {#if skills}
    <div style="margin-bottom: 32px;">
      <h2 style="margin-bottom: 16px;">Your Skills</h2>
      <div style="display: flex; flex-direction: column; gap: 12px;">
        {#each Object.entries(skills.skills) as [skillId, value]}
          <div>
            <div style="display: flex; justify-content: space-between; margin-bottom: 4px;">
              <span style="font-weight: 500;">{skillNames[skillId] || skillId}</span>
              <span style="color: {getSkillColor(value)}; font-weight: bold;">
                {(value * 100).toFixed(0)}%
              </span>
            </div>
            <div style="width: 100%; height: 8px; background: #e0e0e0; border-radius: 4px; overflow: hidden;">
              <div
                style="width: {(value * 100)}%; height: 100%; background: {getSkillColor(value)}; transition: width 0.3s;"
              ></div>
            </div>
          </div>
        {/each}
      </div>
    </div>

    {#if plan}
      <div style="padding: 16px; background: #e8f5e9; border-left: 4px solid #4caf50; border-radius: 4px; margin-bottom: 24px;" class="plan-box">
        <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 12px;">
          <h3 style="margin: 0;" class="plan-title">Today's Plan</h3>
          <div style="display: flex; gap: 8px;">
            {#if isPlanExpired()}
              <span style="color: #f44336; font-size: 12px;" class="plan-expired">Expired</span>
            {/if}
            <button
              onclick={refreshPlan}
              disabled={loading}
              style="padding: 6px 12px; background-color: #4caf50; color: white; border: none; border-radius: 4px; cursor: pointer; font-size: 14px;"
            >
              Refresh Plan
            </button>
          </div>
        </div>
        {#if plan.tasks.length === 0}
          <p style="margin: 0;" class="plan-text">All tasks completed! Great job!</p>
        {:else}
          <ul style="margin: 0; padding-left: 20px;" class="plan-tasks">
            {#each plan.tasks as task}
              <li style="margin-bottom: 8px;" class="plan-task-item">{formatTask(task)}</li>
            {/each}
          </ul>
        {/if}
        <p style="margin: 12px 0 0 0; font-size: 12px;" class="plan-timestamp">
          Generated: {new Date(plan.generated_at * 1000).toLocaleString()}
        </p>
      </div>
    {:else}
      <div style="padding: 16px; background: #fff3e0; border-left: 4px solid #ff9800; border-radius: 4px; margin-bottom: 24px;" class="no-plan-box">
        <div style="display: flex; justify-content: space-between; align-items: center;">
          <div>
            <h3 style="margin: 0 0 8px 0;" class="no-plan-title">No Daily Plan</h3>
            <p style="margin: 0;" class="no-plan-text">Generate a personalized 24-hour curriculum plan.</p>
          </div>
          <button
            onclick={refreshPlan}
            disabled={loading}
            style="padding: 8px 16px; background-color: #ff9800; color: white; border: none; border-radius: 4px; cursor: pointer;"
          >
            Generate Plan
          </button>
        </div>
      </div>
    {/if}

    {#if getWeakestSkill()}
      {@const [weakestId, weakestValue] = getWeakestSkill()!}
      <div style="padding: 16px; background: #fff3e0; border-left: 4px solid #ff9800; border-radius: 4px; margin-bottom: 24px;" class="weakest-skill-box">
        <h3 style="margin-top: 0;" class="weakest-skill-title">Weakest Skill</h3>
        <p style="margin: 0;" class="weakest-skill-text">
          <strong>{skillNames[weakestId] || weakestId}</strong> is your weakest area at {(weakestValue * 100).toFixed(0)}%.
          Focus on improving this skill!
        </p>
      </div>
    {/if}

    <div style="margin-bottom: 24px; display: flex; gap: 12px; flex-wrap: wrap;">
      <button
        onclick={getTargetedProblem}
        disabled={loading}
        style="padding: 12px 24px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500; font-size: 16px;"
      >
        Get Targeted Problem to Improve
      </button>
    </div>

    {#if recommendedProblem}
      <div style="padding: 16px; background: #e3f2fd; border-left: 4px solid #2196f3; border-radius: 4px;" class="recommended-problem-box">
        <h3 style="margin-top: 0;" class="recommended-problem-title">Recommended Problem</h3>
        <p style="margin: 0 0 12px 0;" class="recommended-problem-header">
          <strong>{recommendedProblem.id}</strong> - {skillNames[recommendedProblem.topic] || recommendedProblem.topic}
        </p>
        <p style="margin: 0 0 16px 0; white-space: pre-wrap; line-height: 1.6;" class="recommended-problem-statement">
          {recommendedProblem.statement}
        </p>
        <button
          onclick={() => goto(`/solve?problem=${recommendedProblem.id}&source=recommended`)}
          style="padding: 8px 16px; background-color: #2196f3; color: white; border: none; border-radius: 4px; cursor: pointer;"
        >
          Start Solving
        </button>
      </div>
    {/if}
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

  .recommended-problem-box {
    background: #e3f2fd;
  }

  .recommended-problem-title {
    color: #1976d2;
  }

  .recommended-problem-header {
    color: #212121;
  }

  .recommended-problem-statement {
    color: #212121;
  }

  /* Plan and weakest skill boxes */
  .plan-box {
    background: #e8f5e9;
  }

  .plan-title {
    color: #1b5e20;
  }

  .plan-text {
    color: #212121;
  }

  .plan-task-item {
    color: #212121;
  }

  .plan-timestamp {
    color: #212121;
  }

  .plan-expired {
    color: #f44336;
  }

  .no-plan-box {
    background: #fff3e0;
  }

  .no-plan-title {
    color: #e65100;
  }

  .no-plan-text {
    color: #212121;
  }

  .weakest-skill-box {
    background: #fff3e0;
  }

  .weakest-skill-title {
    color: #e65100;
  }

  .weakest-skill-text {
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

    .recommended-problem-box {
      background: #e3f2fd;
      border-left-color: #2196f3;
    }

    .recommended-problem-title {
      color: #1976d2;
    }

    .recommended-problem-header {
      color: #212121;
    }

    .recommended-problem-statement {
      color: #212121;
    }

    /* Plan and weakest skill boxes - keep light backgrounds with dark text */
    .plan-box {
      background: #e8f5e9;
    }

    .plan-title {
      color: #1b5e20;
    }

    .plan-text {
      color: #212121;
    }

    .plan-task-item {
      color: #212121;
    }

    .plan-timestamp {
      color: #212121;
    }

    .plan-expired {
      color: #f44336;
    }

    .no-plan-box {
      background: #fff3e0;
    }

    .no-plan-title {
      color: #e65100;
    }

    .no-plan-text {
      color: #212121;
    }

    .weakest-skill-box {
      background: #fff3e0;
    }

    .weakest-skill-title {
      color: #e65100;
    }

    .weakest-skill-text {
      color: #212121;
    }
  }
</style>

