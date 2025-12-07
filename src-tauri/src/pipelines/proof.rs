use serde::{Deserialize, Serialize};
use crate::pipelines::router::TaskType;

// Step 1 Response Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    pub id: String,
    pub text: String,
    pub role: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofIssue {
    pub step_id: String,
    #[serde(rename = "type")]
    pub issue_type: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step1Response {
    pub steps: Vec<ProofStep>,
    pub issues: Vec<ProofIssue>,
    pub questions: Vec<String>,
    pub summary: String,
}

// Step 2 Response Structures
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuestionEvaluation {
    pub question: String,
    pub user_answer: String,
    pub assessment: String,
    pub comment: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Step2Response {
    pub evaluation: Vec<QuestionEvaluation>,
    pub next_tasks: Vec<String>,
    pub needs_revision: bool,
}

pub const SYSTEM_PROMPT: &str = r#"You are a rigorous reasoning analyst for technical problem-solving across ALL mathematical, logical, and computational domains.

You MUST analyze solutions, proofs, derivations, code explanations, and logical arguments in these domains:
- Mathematical proofs and derivations (including pure math, analysis, algebra, topology, geometry, number theory, etc.)
- Proof strategy and logical reasoning (formal logic, proof techniques, argumentation, deductive reasoning)
- Reinforcement Learning (RL) theory and equations
- Machine Learning (ML) theory and proofs
- Code debugging and explanations
- Algorithm correctness proofs
- Analysis and real mathematics
- Competition math problems (Putnam, IMO, etc.)
- Logical reasoning arguments
- Any technical solution attempt

CRITICAL INSTRUCTIONS:
- You MUST analyze ANY solution attempt the user provides, regardless of domain
- Do NOT refuse to analyze mathematical proofs, logical reasoning, or proof strategy problems
- These are ALL within your scope and expertise
- Never say you can only handle computer science topics - you handle ALL technical domains
- Always provide analysis, even for pure mathematics or abstract logical arguments

You never output long essays.
You never ramble.
You never editorialize.
You never refuse to analyze a solution.
You always return clean, structured reasoning.

Your job:
Analyze the user's solution attempt (whether it's a proof, derivation, code explanation, or logical argument).
Identify incorrect steps, unjustified leaps, missing arguments, faulty logic, bugs, or errors.
Return your analysis ONLY in proper JSON.

Rules:
Be short, precise, technical, and rigorous.
NEVER include LaTeX formatting in JSON.
NEVER include commentary outside JSON.
NEVER include markdown.
NEVER refuse to analyze - always provide analysis in JSON format.
NEVER invent steps if the user did not provide them.
If the user submits something incoherent or incomplete, still follow the JSON schema and identify what's present and what's missing.
ALWAYS return valid JSON, even if the input seems unrelated to proofs - extract what reasoning structure exists."#;


pub async fn call_deepseek_step1(
    state: &crate::state::app::AppState,
    user_proof: &str,
) -> Result<Step1Response, crate::error::ZosError> {
    use crate::pipelines::router::zos_query;
    use crate::pipelines::perf;
    
    let _perf = perf::PerfTimer::new("step1_total");
    let prompt_start = std::time::Instant::now();
    
    let user_prompt = build_step1_prompt(user_proof);
    let full_prompt = format!("{}\n\n{}", SYSTEM_PROMPT, user_prompt);
    let prompt_ms = prompt_start.elapsed().as_millis() as u64;
    perf::log_perf("step1_prompt_build", prompt_ms);
    
    let routing_start = std::time::Instant::now();
    let result = zos_query::<Step1Response>(state, TaskType::ProofAnalysis, full_prompt).await;
    let routing_ms = routing_start.elapsed().as_millis() as u64;
    perf::log_perf("step1_routing", routing_ms);
    
    result.map_err(|e| e.with_context("Step1 analysis failed"))
}

pub async fn call_deepseek_step2(
    state: &crate::state::app::AppState,
    original_proof: &str,
    issues_json: &str,
    questions: &str,
    user_answers: &str,
) -> Result<Step2Response, crate::error::ZosError> {
    use crate::pipelines::router::zos_query;
    use crate::pipelines::perf;
    
    let _perf = perf::PerfTimer::new("step2_total");
    let prompt_start = std::time::Instant::now();
    
    let user_prompt = build_step2_prompt(original_proof, issues_json, questions, user_answers);
    let full_prompt = format!("{}\n\n{}", SYSTEM_PROMPT, user_prompt);
    let prompt_ms = prompt_start.elapsed().as_millis() as u64;
    perf::log_perf("step2_prompt_build", prompt_ms);
    
    let routing_start = std::time::Instant::now();
    let result = zos_query::<Step2Response>(state, TaskType::ProofAnalysis, full_prompt).await;
    let routing_ms = routing_start.elapsed().as_millis() as u64;
    perf::log_perf("step2_routing", routing_ms);
    
    result.map_err(|e| e.with_context("Step2 evaluation failed"))
}

pub fn build_step1_prompt(user_proof: &str) -> String {
    format!(
        r#"Analyze the following solution attempt. This could be:
- A mathematical proof or derivation (including pure math, analysis, algebra, topology, etc.)
- A proof strategy or logical reasoning argument
- An RL/ML theory explanation
- A code debugging explanation
- An algorithm correctness argument
- A logical reasoning chain
- Any technical solution attempt

IMPORTANT: You MUST analyze this solution attempt regardless of its domain. Do NOT refuse analysis for mathematical proofs, logical reasoning, or proof strategy problems.

Extract its structure and critique it.
Return ONLY valid JSON in the following schema:

{{
  "steps": [
    {{
      "id": "s1",
      "text": "the user's first meaningful statement or step",
      "role": "assumption | deduction | claim | definition | conclusion | code_statement | explanation"
    }}
  ],
  "issues": [
    {{
      "step_id": "s1",
      "type": "missing_justification | faulty_logic | misuse_of_theorem | undefined_term | code_bug | incorrect_derivation | logical_error",
      "explanation": "short explanation"
    }}
  ],
  "questions": [
    "generate 2–3 clarifying questions the user should answer"
  ],
  "summary": "short rigorous overview of the solution's main problems or strengths"
}}

Requirements:
Output only valid JSON.
Be precise, concise, and technical.
Follow the user's solution structure; do not invent nonexistent steps.
If the input is code, treat code statements as steps.
If the input is a derivation, treat each equation manipulation as a step.
If the input is a mathematical proof, analyze each logical step.
If the input is a logical reasoning argument, analyze the argument structure.
If unsure how to interpret something, ask a clarifying question in "questions".
ALWAYS return valid JSON - if the input seems incomplete, identify what's present and what's missing.
NEVER refuse to analyze - always provide analysis in the JSON format.

User's solution attempt:

{}"#,
        user_proof
    )
}

pub fn build_step2_prompt(
    original_proof: &str,
    issues_json: &str,
    questions: &str,
    user_answers: &str,
) -> String {
    format!(
        r#"You will continue a Socratic solution-improvement dialogue.

You are given:
- the user's original solution attempt (proof, derivation, code explanation, etc.)
- the structured issues you identified
- the clarifying questions you asked
- the user's answers to those questions

Your job now:
- Evaluate the user's answers
- Determine whether they fix each issue
- Produce next-step guidance
- Return ONLY valid JSON in the schema:

{{
  "evaluation": [
    {{
      "question": "the original question",
      "user_answer": "the user's response",
      "assessment": "correct | partially_correct | incorrect | unclear",
      "comment": "1–2 sentence explanation"
    }}
  ],
  "next_tasks": [
    "one or two specific instructions the user should do next"
  ],
  "needs_revision": true
}}

Rules:
Be technically rigorous but concise.
Keep comments extremely short.
Output only JSON.
Evaluate answers in context of the solution domain (math, code, RL, ML, etc.).

Inputs:

Original solution:
{}

Issues detected:
{}

Clarifying questions:
{}

User answers:
{}"#,
        original_proof, issues_json, questions, user_answers
    )
}
