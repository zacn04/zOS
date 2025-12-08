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
        r#"Analyze this solution attempt and return ONLY valid JSON:

{{
  "steps": [{{"id": "s1", "text": "...", "role": "assumption|deduction|claim|definition|conclusion|code_statement|explanation"}}],
  "issues": [{{"step_id": "s1", "type": "missing_justification|faulty_logic|misuse_of_theorem|undefined_term|code_bug|incorrect_derivation|logical_error", "explanation": "..."}}],
  "questions": ["..."],
  "summary": "..."
}}

Example: {{"steps": [{{"id": "s1", "text": "Assume P", "role": "assumption"}}], "issues": [], "questions": ["Why P?"], "summary": "Basic assumption"}}

Return ONLY JSON, no markdown, no explanations.

Solution attempt:
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
        r#"Evaluate answers and return ONLY valid JSON:

{{
  "evaluation": [{{"question": "...", "user_answer": "...", "assessment": "correct|partially_correct|incorrect|unclear", "comment": "..."}}],
  "next_tasks": ["..."],
  "needs_revision": true
}}

Example: {{"evaluation": [{{"question": "Why P?", "user_answer": "Because Q", "assessment": "correct", "comment": "Valid reasoning"}}], "next_tasks": ["Prove Q"], "needs_revision": false}}

Return ONLY JSON, no markdown, no explanations.

Original: {}
Issues: {}
Questions: {}
Answers: {}"#,
        original_proof, issues_json, questions, user_answers
    )
}
