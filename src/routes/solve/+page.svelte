<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { goto } from "$app/navigation";
  import { page } from "$app/stores";

  type Problem = {
    id: string;
    topic: string;
    difficulty: number;
    statement: string;
    solution_sketch: string;
  };

  type ProofStep = {
    id: string;
    text: string;
    role: string;
  };

  type ProofIssue = {
    step_id: string;
    type: string;
    explanation: string;
  };

  type Step1Response = {
    steps: ProofStep[];
    issues: ProofIssue[];
    questions: string[];
    summary: string;
  };

  type QuestionEvaluation = {
    question: string;
    user_answer: string;
    assessment: string;
    comment: string;
  };

  type Step2Response = {
    evaluation: QuestionEvaluation[];
    next_tasks: string[];
    needs_revision: boolean;
  };

  let currentProblem = $state<Problem | null>(null);
  let step = $state(0); // 0 = show problem, 1 = view analysis, 2 = answer questions, 3 = view evaluation
  let proof = $state("");
  let step1Result = $state<Step1Response | null>(null);
  let answers = $state<string[]>([]);
  let step2Result = $state<Step2Response | null>(null);
  let error = $state("");
  let loading = $state(false);

  async function getRecommendedProblem() {
    try {
      loading = true;
      error = "";
      const problem = await invoke<Problem>("get_recommended_problem");
      currentProblem = problem;
      proof = "";
      step = 0;
      step1Result = null;
      answers = [];
      step2Result = null;
      
      // Propagate problem ID to URL
      goto(`/solve?problem=${problem.id}`, { replaceState: true });
      
      // Trigger precomputation of next problem in background (don't await)
      invoke("precompute_next_problem").catch(err => {
        console.warn("Failed to precompute next problem:", err);
      });
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function analyzeProof() {
    if (!proof.trim()) {
      error = "Please enter your solution/proof.";
      return;
    }

    try {
      loading = true;
      error = "";
      const res = await invoke<Step1Response>("step1_analyze_proof", { 
        proof,
        problem_id: currentProblem?.id || null,
        problem_topic: currentProblem?.topic || null,
        problem_difficulty: currentProblem?.difficulty || null
      });
      step1Result = res;
      answers = Array(res.questions.length).fill("");
      step = 1;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function proceedToAnswers() {
    if (step1Result && step1Result.questions.length > 0) {
      // Save attempt when proceeding to answer questions (counts as submission)
      if (currentProblem && proof.trim()) {
        try {
          await invoke("submit_problem_attempt", {
            problem_id: currentProblem.id,
            problem_topic: currentProblem.topic,
            problem_difficulty: currentProblem.difficulty,
            user_attempt: proof,
            status: "answering_questions"
          });
        } catch (err) {
          console.warn("Failed to save attempt when proceeding to questions:", err);
        }
      }
      step = 2;
    }
  }

  async function submitAnswers() {
    if (!step1Result) return;

    const emptyAnswers = answers.filter(a => !a.trim());
    if (emptyAnswers.length > 0) {
      error = "Please answer all questions before submitting.";
      return;
    }

    try {
      loading = true;
      error = "";
      
      const res = await invoke<Step2Response>("step2_evaluate_answers", {
        proof,
        issues: step1Result.issues,
        questions: step1Result.questions,
        answers,
        problem_id: currentProblem?.id || null,
        problem_topic: currentProblem?.topic || null,
        problem_difficulty: currentProblem?.difficulty || null,
      });
      
      step2Result = res;
      step = 3;
    } catch (err) {
      error = String(err);
    } finally {
      loading = false;
    }
  }

  async function startNewProblem() {
    // Save current attempt if we have one that hasn't been saved yet
    // (Perfect solutions and step2 already save records, so we only save if user is abandoning)
    if (currentProblem && proof.trim()) {
      // Only save if we haven't already saved (step1Result with perfect solution or step2Result already saved)
      const alreadySaved = (step1Result && step1Result.issues.length === 0 && step1Result.questions.length === 0) || step2Result;
      if (!alreadySaved) {
        try {
          await invoke("submit_problem_attempt", {
            problem_id: currentProblem.id,
            problem_topic: currentProblem.topic,
            problem_difficulty: currentProblem.difficulty,
            user_attempt: proof,
            status: "abandoned"
          });
        } catch (err) {
          console.warn("Failed to save abandoned attempt:", err);
        }
      }
    }
    getRecommendedProblem();
  }

  async function tryAnotherSolution() {
    // Save current attempt before trying again
    if (currentProblem && proof.trim()) {
      try {
        await invoke("submit_problem_attempt", {
          problem_id: currentProblem.id,
          problem_topic: currentProblem.topic,
          problem_difficulty: currentProblem.difficulty,
          user_attempt: proof,
          status: "retrying"
        });
      } catch (err) {
        console.warn("Failed to save attempt before retry:", err);
      }
    }
    // Reset to step 0 to try again
    step = 0;
    proof = "";
    step1Result = null;
    answers = [];
    step2Result = null;
  }

  function getRoleClass(role: string): string {
    return `step-${role}`;
  }

  function getIssueTypeClass(type: string): string {
    return `issue-${type.replace(/_/g, "-")}`;
  }

  function getAssessmentClass(assessment: string): string {
    const classes: Record<string, string> = {
      correct: "assessment-correct",
      partially_correct: "assessment-partial",
      incorrect: "assessment-incorrect",
      unclear: "assessment-unclear",
    };
    return classes[assessment] || "assessment-unclear";
  }

  function getAssessmentColor(assessment: string): string {
    const colors: Record<string, string> = {
      correct: "#4caf50",
      partially_correct: "#ff9800",
      incorrect: "#f44336",
      unclear: "#9e9e9e",
    };
    return colors[assessment] || "#9e9e9e";
  }

  // Function to load problem by ID
  async function loadProblemById(problemId: string) {
    if (loading) return; // Prevent concurrent loads
    loading = true;
    error = "";
    try {
      const problem = await invoke<Problem>("get_problem_by_id", { problemId: problemId });
      currentProblem = problem;
      proof = "";
      step = 0;
      step1Result = null;
      answers = [];
      step2Result = null;
      
      // Propagate problem ID to URL
      goto(`/solve?problem=${problem.id}`, { replaceState: true });
      
      loading = false;
      
      // Trigger precomputation of next problem in background (don't await)
      invoke("precompute_next_problem").catch(err => {
        console.warn("Failed to precompute next problem:", err);
      });
    } catch (err) {
      
      error = String(err);
      loading = false;
      // Fall back to recommended problem if loading by ID fails
      getRecommendedProblem();
    }
  }

  // Watch for URL changes reactively
  $effect(() => {
    const problemId = $page.url.searchParams.get("problem");
    // console.log("current problem", currentProblem?.id);
    // console.log("propagated problem", problemId);
    if (currentProblem?.id === undefined && problemId) {
      // console.log("loading problem by ID", problemId)
      loadProblemById(problemId);
    }
    if (problemId && problemId !== currentProblem?.id && !loading) {
      loadProblemById(problemId);
      // Problem ID is propagated to URL in loadProblemById, so we keep it there
    } else if (!problemId && !currentProblem && !loading) {
      // Only load recommended problem if we don't have a problem and no ID in URL
      getRecommendedProblem();
    }
  });
</script>

<div style="padding: 24px; font-family: sans-serif; max-width: 1200px; margin: 0 auto;">
  <div style="display: flex; justify-content: space-between; align-items: center; margin-bottom: 24px;">
    <h1 style="margin: 0;">Solve</h1>
    <div style="display: flex; gap: 12px;">
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
    <div style="padding: 16px; text-align: center;">Loading...</div>
  {/if}

  {#if step === 0 && currentProblem}
    <div>
      <div style="margin-bottom: 16px;" class="problem-metadata">
        <span class="problem-category">
          {currentProblem.topic.replace(/_/g, " ")}
        </span>
        <span class="problem-difficulty-text">
          Difficulty: {(currentProblem.difficulty * 100).toFixed(0)}%
        </span>
      </div>

      <div style="padding: 16px; background: #f5f5f5; border-radius: 4px; margin-bottom: 16px;" class="problem-statement-box">
        <h2 style="margin-top: 0;" class="problem-statement-title">Problem Statement</h2>
        <p style="white-space: pre-wrap; line-height: 1.6;" class="problem-statement-text">{currentProblem.statement}</p>
      </div>

      <div>
        <h3>Your Solution/Proof:</h3>
        <textarea
          bind:value={proof}
          placeholder="Enter your solution, proof, or derivation here..."
          style="width: 100%; height: 300px; margin-bottom: 12px; padding: 8px; border: 1px solid #ccc; border-radius: 4px; font-family: monospace; box-sizing: border-box;"
        />
        <button
          on:click={analyzeProof}
          disabled={loading}
          style="padding: 8px 16px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
        >
          Analyze Solution
        </button>
      </div>
    </div>
  {/if}

  <!-- Step 1: View Analysis -->
  {#if step === 1 && step1Result}
    <div style="margin-top: 24px;">
      <!-- Summary -->
      <div class="summary-box">
        <h2 class="summary-title">Analysis Summary</h2>
        <p class="summary-text">{step1Result.summary}</p>
      </div>

      <!-- Proof Steps -->
      <div style="margin-bottom: 24px;">
        <h2 style="margin-bottom: 12px; font-size: 18px;">Solution Steps</h2>
        <div style="display: flex; flex-direction: column; gap: 8px;">
          {#each step1Result.steps as stepItem}
            <div class="step-box {getRoleClass(stepItem.role)}">
              <div class="step-header">
                <span class="step-id">{stepItem.id}</span>
                <span class="step-role-badge">
                  {stepItem.role}
                </span>
              </div>
              <p class="step-text">{stepItem.text}</p>
            </div>
          {/each}
        </div>
      </div>

      <!-- Issues -->
      {#if step1Result.issues.length > 0}
        <div style="margin-bottom: 24px;">
          <h2 style="margin-bottom: 12px; font-size: 18px; color: #d32f2f;">Issues Found</h2>
          <div style="display: flex; flex-direction: column; gap: 8px;">
            {#each step1Result.issues as issue}
              <div class="issue-box {getIssueTypeClass(issue.type)}">
                <div class="issue-header">
                  <span class="issue-step-id">Step {issue.step_id}</span>
                  <span class="issue-type-badge">
                    {issue.type.replace(/_/g, " ")}
                  </span>
                </div>
                <p class="issue-text">{issue.explanation}</p>
              </div>
            {/each}
          </div>
        </div>
      {/if}

      <!-- Questions -->
      {#if step1Result.questions.length > 0}
        <div style="margin-bottom: 24px;">
          <h2 style="margin-bottom: 12px; font-size: 18px; color: #f57c00;">Clarifying Questions</h2>
          <ol class="questions-list">
            {#each step1Result.questions as question}
              <li class="question-item">
                {question}
              </li>
            {/each}
          </ol>
        </div>
      {/if}

      <!-- Perfect Proof Message -->
      {#if step1Result.issues.length === 0 && step1Result.questions.length === 0}
        <div style="padding: 16px; background: #e8f5e9; border-left: 4px solid #4caf50; border-radius: 4px; margin-bottom: 16px;">
          <h3 style="margin-top: 0; color: #1b5e20;">✓ Perfect Solution!</h3>
          <p style="margin: 0; color: #212121;">Your solution is correct with no issues found. Great work!</p>
        </div>
      {/if}

      {#if step1Result.questions.length > 0}
        <button
          on:click={proceedToAnswers}
          style="padding: 8px 16px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500; margin-top: 16px;"
        >
          Answer Questions
        </button>
      {:else}
        <div style="display: flex; gap: 12px; margin-top: 16px;">
          <button
            on:click={startNewProblem}
            style="padding: 8px 16px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
          >
            Get New Problem
          </button>
          <button
            on:click={tryAnotherSolution}
            style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
          >
            Try Another Solution
          </button>
        </div>
      {/if}
    </div>
  {/if}

  <!-- Step 2: Answer Questions -->
  {#if step === 2 && step1Result}
    <div style="margin-top: 24px;">
      <h2 style="margin-bottom: 16px; font-size: 20px;">Answer Clarifying Questions</h2>
      
      <div style="display: flex; flex-direction: column; gap: 16px; margin-bottom: 24px;">
        {#each step1Result.questions as question, index}
          <div class="question-answer-box">
            <div style="margin-bottom: 8px;">
              <strong style="color: #f57c00;">Question {index + 1}:</strong>
              <p style="margin: 4px 0 8px 0; line-height: 1.6;">{question}</p>
            </div>
            <textarea
              bind:value={answers[index]}
              placeholder="Your answer..."
              class="answer-input"
              class:answer-empty={!answers[index] || !answers[index].trim()}
            />
          </div>
        {/each}
      </div>

      <div style="display: flex; gap: 12px;">
        <button
          on:click={() => step = 1}
          style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
        >
          Back
        </button>
        <button
          on:click={submitAnswers}
          disabled={loading}
          style="padding: 8px 16px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
        >
          Submit Answers
        </button>
      </div>
    </div>
  {/if}

  <!-- Step 3: View Evaluation -->
  {#if step === 3 && step2Result}
    <div style="margin-top: 24px;">
      <h2 style="margin-bottom: 16px; font-size: 20px;">Evaluation Results</h2>

      <!-- Evaluation Cards -->
      <div style="margin-bottom: 24px;">
        <h3 style="margin-bottom: 12px; font-size: 18px;">Question Assessments</h3>
        <div style="display: flex; flex-direction: column; gap: 12px;">
          {#each step2Result.evaluation as evaluation}
            <div class="evaluation-card {getAssessmentClass(evaluation.assessment)}">
              <div class="evaluation-header">
                <div>
                  <strong>Question:</strong> {evaluation.question}
                </div>
                <span
                  class="assessment-badge"
                  style="background-color: {getAssessmentColor(evaluation.assessment)};"
                >
                  {evaluation.assessment.replace(/_/g, " ")}
                </span>
              </div>
              <div style="margin-top: 8px;">
                <strong>Your Answer:</strong> {evaluation.user_answer}
              </div>
              <div style="margin-top: 8px; padding: 8px; background: rgba(0,0,0,0.05); border-radius: 4px;">
                <strong>Comment:</strong> {evaluation.comment}
              </div>
            </div>
          {/each}
        </div>
      </div>

      <!-- Next Tasks -->
      {#if step2Result.next_tasks.length > 0}
        <div style="margin-bottom: 24px;">
          <h3 style="margin-bottom: 12px; font-size: 18px;">Next Steps</h3>
          <ul class="tasks-list">
            {#each step2Result.next_tasks as task}
              <li class="task-item">
                {task}
              </li>
            {/each}
          </ul>
        </div>
      {/if}

      <!-- Revision Status -->
      {#if step2Result.needs_revision}
        <div class="revision-notice">
          <strong>⚠️ Revision Required:</strong> Your solution needs revision based on the evaluation.
        </div>
      {/if}

      <div style="display: flex; gap: 12px; margin-top: 24px;">
        <button
          on:click={startNewProblem}
          style="padding: 8px 16px; background-color: #396cd8; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
        >
          Get New Problem
        </button>
        <button
          on:click={tryAnotherSolution}
          style="padding: 8px 16px; background-color: #757575; color: white; border: none; border-radius: 4px; cursor: pointer; font-weight: 500;"
        >
          Try Again
        </button>
      </div>
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

  /* Summary Box */
  .summary-box {
    margin-bottom: 24px;
    padding: 16px;
    background: #e3f2fd;
    border-left: 4px solid #2196f3;
    border-radius: 4px;
  }

  .summary-title {
    margin-top: 0;
    margin-bottom: 8px;
    font-size: 18px;
    color: #1976d2;
  }

  .summary-text {
    margin: 0;
    line-height: 1.6;
    color: #000;
  }

  /* Step Boxes */
  .step-box {
    padding: 12px;
    border-radius: 4px;
    border-left: 4px solid #666;
  }

  .step-box.step-assumption {
    background: #e3f2fd;
  }

  .step-box.step-deduction {
    background: #f3e5f5;
  }

  .step-box.step-claim {
    background: #fff3e0;
  }

  .step-box.step-definition {
    background: #e8f5e9;
  }

  .step-box.step-conclusion {
    background: #fce4ec;
  }

  .step-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .step-id {
    font-weight: bold;
    color: #666;
  }

  .step-role-badge {
    font-size: 12px;
    padding: 2px 8px;
    background: rgba(0, 0, 0, 0.1);
    border-radius: 12px;
    text-transform: capitalize;
  }

  .step-text {
    margin: 0;
    line-height: 1.5;
    color: #000;
  }

  /* Issue Boxes */
  .issue-box {
    padding: 12px;
    border-radius: 4px;
    border-left: 4px solid #d32f2f;
  }

  .issue-box.issue-missing-justification {
    background: #ffebee;
  }

  .issue-box.issue-faulty-logic {
    background: #fff3e0;
  }

  .issue-box.issue-misuse-of-theorem {
    background: #fce4ec;
  }

  .issue-box.issue-undefined-term {
    background: #e8f5e9;
  }

  .issue-header {
    display: flex;
    align-items: center;
    gap: 8px;
    margin-bottom: 4px;
  }

  .issue-step-id {
    font-weight: bold;
    color: #d32f2f;
  }

  .issue-type-badge {
    font-size: 12px;
    padding: 2px 8px;
    background: rgba(211, 47, 47, 0.2);
    border-radius: 12px;
    text-transform: capitalize;
  }

  .issue-text {
    margin: 0;
    line-height: 1.5;
    color: #000;
  }

  /* Questions */
  .questions-list {
    margin: 0;
    padding-left: 20px;
  }

  .question-item {
    margin-bottom: 8px;
    line-height: 1.6;
    padding: 8px;
    background: #fff3e0;
    border-radius: 4px;
    color: #000;
  }

  /* Question Answer Box */
  .question-answer-box {
    padding: 16px;
    background: #fff;
    border: 1px solid #ddd;
    border-radius: 4px;
  }

  .answer-input {
    width: 100%;
    min-height: 80px;
    padding: 8px;
    border: 1px solid #ccc;
    border-radius: 4px;
    font-family: sans-serif;
    box-sizing: border-box;
    resize: vertical;
  }

  .answer-input.answer-empty {
    border-color: #f44336;
    border-width: 2px;
  }

  /* Evaluation Cards */
  .evaluation-card {
    padding: 16px;
    border-radius: 4px;
    border-left: 4px solid;
  }

  .evaluation-card.assessment-correct {
    background: #e8f5e9;
    border-left-color: #4caf50;
  }

  .evaluation-card.assessment-partial {
    background: #fff3e0;
    border-left-color: #ff9800;
  }

  .evaluation-card.assessment-incorrect {
    background: #ffebee;
    border-left-color: #f44336;
  }

  .evaluation-card.assessment-unclear {
    background: #f5f5f5;
    border-left-color: #9e9e9e;
  }

  .evaluation-header {
    display: flex;
    justify-content: space-between;
    align-items: flex-start;
    gap: 12px;
    margin-bottom: 8px;
  }

  .assessment-badge {
    padding: 4px 12px;
    border-radius: 12px;
    color: white;
    font-size: 12px;
    font-weight: bold;
    text-transform: capitalize;
    white-space: nowrap;
  }

  /* Tasks List */
  .tasks-list {
    margin: 0;
    padding-left: 20px;
    list-style: none;
  }

  .task-item {
    margin-bottom: 8px;
    padding: 12px;
    background: #e3f2fd;
    border-left: 4px solid #2196f3;
    border-radius: 4px;
    line-height: 1.6;
  }

  .task-item::before {
    content: "✓ ";
    color: #2196f3;
    font-weight: bold;
    margin-right: 8px;
  }

  /* Revision Notice */
  .revision-notice {
    padding: 12px;
    background: #fff3e0;
    border: 1px solid #ff9800;
    border-radius: 4px;
    color: #e65100;
    margin-bottom: 16px;
  }

  /* Error Box */
  .error-box {
    margin-top: 24px;
    padding: 12px;
    background: #ffebee;
    border: 1px solid #f44336;
    border-radius: 4px;
    color: #c62828;
  }

  /* Problem Statement Box */
  .problem-statement-box {
    background: #ffffff;
    border: 1px solid #e0e0e0;
  }

  .problem-statement-title {
    color: #212121;
  }

  .problem-statement-text {
    color: #212121;
  }

  /* Problem metadata */
  .problem-metadata {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .problem-category {
    padding: 4px 12px;
    background: #e3f2fd;
    border-radius: 12px;
    font-size: 12px;
    text-transform: capitalize;
    color: #1976d2;
    font-weight: 500;
  }

  .problem-difficulty-text {
    font-size: 14px;
    color: #212121;
    font-weight: 500;
  }

  /* Dark Mode */
  @media (prefers-color-scheme: dark) {
    :global(body) {
      background-color: #1e1e1e;
      color: #e0e0e0;
    }

    :global(div) {
      color: #e0e0e0;
    }

    h1, h2, h3 {
      color: #ffffff !important;
    }

    textarea {
      background-color: #2d2d2d !important;
      color: #e0e0e0 !important;
      border-color: #444 !important;
    }

    button {
      background-color: #396cd8 !important;
    }

    button:hover {
      background-color: #2d5aa0 !important;
    }

    /* Dark mode summary */
    .summary-box {
      background: #1a237e;
      border-left-color: #3f51b5;
    }

    .summary-title {
      color: #9fa8da;
    }

    .summary-text {
      color: #e0e0e0;
    }

    /* Dark mode steps */
    .step-box.step-assumption {
      background: #1a237e;
      border-left-color: #3f51b5;
    }

    .step-box.step-deduction {
      background: #4a148c;
      border-left-color: #7b1fa2;
    }

    .step-box.step-claim {
      background: #e65100;
      border-left-color: #ff6f00;
    }

    .step-box.step-definition {
      background: #1b5e20;
      border-left-color: #388e3c;
    }

    .step-box.step-conclusion {
      background: #880e4f;
      border-left-color: #c2185b;
    }

    .step-id {
      color: #b0bec5;
    }

    .step-role-badge {
      background: rgba(255, 255, 255, 0.2);
      color: #fff;
    }

    .step-text {
      color: #e0e0e0;
    }

    /* Dark mode issues */
    .issue-box.issue-missing-justification {
      background: #b71c1c;
      border-left-color: #d32f2f;
    }

    .issue-box.issue-faulty-logic {
      background: #e65100;
      border-left-color: #ff6f00;
    }

    .issue-box.issue-misuse-of-theorem {
      background: #880e4f;
      border-left-color: #c2185b;
    }

    .issue-box.issue-undefined-term {
      background: #1b5e20;
      border-left-color: #388e3c;
    }

    .issue-step-id {
      color: #ffcdd2;
    }

    .issue-type-badge {
      background: rgba(255, 255, 255, 0.2);
      color: #fff;
    }

    .issue-text {
      color: #e0e0e0;
    }

    /* Dark mode questions */
    .question-item {
      background: #e65100;
      color: #fff;
    }

    /* Dark mode question answer box */
    .question-answer-box {
      background: #2d2d2d;
      border-color: #444;
    }

    .answer-input {
      background-color: #1e1e1e !important;
      color: #e0e0e0 !important;
      border-color: #555 !important;
    }

    .answer-input.answer-empty {
      border-color: #f44336 !important;
    }

    /* Dark mode evaluation cards */
    .evaluation-card.assessment-correct {
      background: #1b5e20;
      border-left-color: #4caf50;
    }

    .evaluation-card.assessment-partial {
      background: #e65100;
      border-left-color: #ff9800;
    }

    .evaluation-card.assessment-incorrect {
      background: #b71c1c;
      border-left-color: #f44336;
    }

    .evaluation-card.assessment-unclear {
      background: #424242;
      border-left-color: #9e9e9e;
    }

    .evaluation-card {
      color: #e0e0e0;
    }

    /* Dark mode tasks */
    .task-item {
      background: #1a237e;
      border-left-color: #3f51b5;
      color: #e0e0e0;
    }

    .task-item::before {
      color: #9fa8da;
    }

    /* Dark mode revision notice */
    .revision-notice {
      background: #e65100;
      border-color: #ff9800;
      color: #fff;
    }

    /* Dark mode error */
    .error-box {
      background: #b71c1c;
      border-color: #d32f2f;
      color: #ffcdd2;
    }

    /* Dark mode problem statement - use light background for readability */
    .problem-statement-box {
      background: #ffffff;
      border-color: #e0e0e0;
    }

    .problem-statement-title {
      color: #212121 !important;
    }

    .problem-statement-text {
      color: #212121 !important;
    }

    /* Problem metadata in dark mode - keep readable */
    .problem-category {
      background: #e3f2fd;
      color: #1976d2;
    }

    .problem-difficulty-text {
      color: #ffffff;
    }
  }
</style>

