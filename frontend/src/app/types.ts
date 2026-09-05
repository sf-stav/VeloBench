// Shared types mirroring the backend JSON.

export interface Settings {
  providers: Provider[];
  active_provider_id: string | null;
  default_config?: ConfigRef | null;
  helper: HelperConfig | null;
  max_stats_tokens: number; // live-stats memory budget (tokens), truncated when exceeded
  max_graph_points: number; // point cap for the live graphs; beyond this they slide (oldest dropped)
  intra_token_latency_split_cap_ms: number;
  session_categories: string[]; // managed tags for sessions (Sessions page filter) // upper limit for the intra-token-latency bimodal split (default 11 ms)
  telemetry: TelemetryConfig;
}

/** Mini-OTel receiver config (Settings → Telemetry). Default OFF. */
export interface TelemetryConfig {
  enabled: boolean;
  host: string;   // default 0.0.0.0
  port: number;   // default 9381
  max_streams: number;    // distinct streams displayed at once (1..8)
  chat_lines: number;     // mini-chat sliding window, in lines
  record_max_secs: number;   // recording auto-stop (hard cap 300)
  record_max_tokens: number; // recording auto-stop (hard cap 20000)
  stats_max_tokens: number;  // telemetry-only chart-data budget: the data behind the charts slides once a stream exceeds this (independent of the chat live-stats threshold)
}

/** Live snapshot of the telemetry receiver (polled by the Telemetry page). */
export interface TelemetryState {
  clientConnected: boolean;
  metricPoints: number;
  status: { model: string; topology: string } | null;
  streams: TelemetryStream[];
  config: TelemetryConfig;
}

export interface TelemetryStream {
  requestId: string;
  generationId: string;
  model: string;
  topology: string;
  done: boolean;
  finishReason: string | null;
  startedMs: number;
  lastMs: number;
  text: string;
  stats: {
    tokS: number; tokens: number; ttftMs: number | null;
    avg: number; median: number; min: number; max: number;
  };
  series: Array<{ t: number; tokS: number }>;
  recording: { elapsedS: number; tokens: number; maxS: number; maxTokens: number } | null;
}

export interface ConfigRef {
  provider_id: string;
  model_id: string;
}

export interface Provider {
  id: string;
  name: string;
  base_url: string;
  api_key?: string;
  models: ModelConfig[];
}

export interface ModelConfig {
  uid?: string;
  label?: string;
  id: string;
  params: ParamOverride[];
  reasoning_enabled: boolean;
  reasoning_effort?: string;
  tokenizer?: string;
  live_calibration?: { ratio: number; weight: number; updated_at?: string };
}

export interface ParamOverride {
  key: string;
  value: any;
}

export interface HelperConfig {
  provider_id?: string;
  base_url: string;
  api_key?: string;
  model: string;
  reasoning_effort?: string;
  params: ParamOverride[];
  /** Max concurrent helper requests during analysis (default 1). */
  concurrency?: number;
}

export interface ChatMessage {
  role: string;
  content: any; // string | multimodal parts
  images?: string[]; // data-URL images (sent as a separate proto field)
  /** Context-fill marker: nominal payload size in tokens. The server replaces
   *  the content with an exact corpus slice of this size before sending. */
  fillTokens?: number;
}

export interface StreamRequest {
  provider_id: string;
  model: string;
  messages: ChatMessage[];
  overrides?: ParamOverride[];
  temperature?: number;
  reasoning_enabled?: boolean;
  reasoning_effort?: string;
  no_stream?: boolean;
}

// ----- classification -----

export interface SegmentProto {
  category: string;
  text: string;
}

export interface ClassifyResponse {
  category: string;
  segments: SegmentProto[];
  raw?: string;
}

// ----- benchmark / stats -----

export interface TokenEvent {
  t_ms: number;
  chars: number;
  est_tokens: number;
  kind: string; // 'content' | 'reasoning'
  text: string;
  regime?: string;
}

export interface Segment {
  category: string;
  start_char: number;
  end_char: number;
  start_event: number;
  end_event: number;
  token_count: number;
  avg_tok_s?: number;
}

export interface GenStats {
  ttft_ms?: number;
  total_ms: number;
  decode_ms: number;
  prompt_tokens?: number;
  completion_tokens?: number;
  content_tokens?: number;
  reasoning_tokens?: number;
  final_tok_s?: number;
  live_avg_tok_s?: number;
  live_min_tok_s?: number;
  live_max_tok_s?: number;
  live_median_tok_s?: number;
  token_events: TokenEvent[];
}

export interface Usage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
  completion_tokens_details?: any;
  prompt_tokens_details?: any;
}

export interface Benchmark {
  id: string;
  created_at: string;
  kind: string;
  label: string;
  provider: string;
  model: string;
  session: string;
  prompt: string;
  reasoning: string;
  output: string;
  category?: string;
  segments: Segment[];
  stats: GenStats;
  usage?: Usage;
  meta: any;
}

// A live sample pushed during streaming (for charting).
export interface LiveSample {
  t_ms: number; // ms since req start
  tok_s: number; // instantaneous rate this event
  kind: string;
  regime?: string;
}

// Bimodality/cluster result (computed server-side; type shared for the charts).
export interface LatencyCluster {
  mean: number;
  count: number;
  std: number;
  min: number;
  max: number;
}
export interface LatencyClusterResult {
  bimodal: boolean;
  split: number;
  eta: number;
  clusters: LatencyCluster[];
  total: number;
}

// ---------- post-hoc session analysis ----------

/** Canonical regimes (mirrors src/analyze.rs REGIMES). `prose` is the free
 * tier's residual label; the helper analysis refines it into subtypes. */
export const ANALYSIS_REGIMES = [
  'code',
  'math',
  'structured',
  'prose',
  'reasoning_prose',
  'creative_prose',
  'other_prose',
] as const;

export type AnalysisRegime = (typeof ANALYSIS_REGIMES)[number] | 'unknown';

export interface SessionAnalysis {
  modelLabel?: string | null;
  session: string;
  created_at: string;
  helper_model: string;
  status: 'running' | 'done' | 'error' | 'free';
  progress: number;
  chunks_done: number;
  chunks_total: number;
  error?: string | null;
  category?: string | null;
  /** Bump when the prompt/alignment scheme changes. */
  version?: number;
  turns: unknown[];
}

/** One generated text event in an analysed transcript. */
export interface TranscriptEvent {
  i: number;
  tMs: number;
  estTokens: number;
  kind: string;
  regime: AnalysisRegime;
  startChar: number;
  endChar: number;
}

/** One section (reasoning | content) of a turn in an analysed transcript. */
export interface TranscriptSection {
  kind: string;
  text: string;
  events: TranscriptEvent[];
}

/** Label provenance for one analysis segment (two-tier scheme). */
export interface AnalysisSegment {
  category: string;
  kind: string;
  /** "free" (deterministic) | "assisted" (helper model). */
  source?: string;
  startChar: number;
  endChar: number;
  startEvent: number;
  endEvent: number;
  tokenCount: number;
}

export interface TranscriptTurn {
  benchmarkId: string;
  createdAt: string;
  model: string;
  provider: string;
  /** Recorded run stats (optional: absent on very old records). */
  ttftMs?: number | null;
  completionTokens?: number | null;
  finalTokS?: number | null;
  liveMedianTokS?: number | null;
  liveMinTokS?: number | null;
  liveMaxTokS?: number | null;
  genMs?: number | null;
  totalMs?: number | null;
  promptTokens?: number | null;
  fillTokens?: number | null;
  tokenSource?: string | null;
  modelLabel?: string | null;
  acceptedPredTokens?: number | null;
  rejectedPredTokens?: number | null;
  reasoningEnabled?: boolean | null;
  reasoningEffort?: string | null;
  /** Test-run bookkeeping (test sessions only). */
  kind?: string;
  label?: string;
  section?: string | null;
  regimesFromSections?: boolean;
  sections: TranscriptSection[];
  /** Regime segments with label provenance (snake_case from the server). */
  segments?: AnalysisSegment[];
}

export interface SessionAnalysisDetail extends SessionAnalysis {
  turns: TranscriptTurn[];
}

/** User-authored session metadata (custom display name + managed category). */
export interface SessionMeta {
  name?: string | null;
  category?: string | null;
}

/** A session row on the Sessions page: the turns sharing one session id. */
export interface SessionGroup {
  session: string;
  createdAt: string;
  turns: Benchmark[];
  model: string;
  provider: string;
  totalTokens: number;
  /** Origin of the session: 'chat' (manual) or 'test' (Test Constructor run). */
  kind: string;
  /** Manual chats: 'manual-chat'. Test runs: the test title. */
  label: string;
}

// ---------- Test Constructor ----------

export type TestStepType = 'section' | 'prompt' | 'context' | 'bench' | 'image';

export interface TestStep {
  type: TestStepType;
  /** Section title ("Section" steps; required). */
  title?: string;
  /** Prompt text ("prompt" steps). */
  text?: string;
  /** Context filler size in kilo-tokens ("context" steps). */
  k?: number;
  /** Sections only: true → start a new LLM session (clear history).
   *  false (default) → just a progress marker, the conversation continues. */
  reset?: boolean;
  /** Image steps: file inside assets/test_images (selected by size). */
  image?: string;
  /** Image steps: prompt sent with the image. */
  prompt?: string;
  /** Bench steps (fixed-shape run): corpus tokens of context depth. */
  depth?: number;
  /** Bench steps: measured prompt tokens on top of the depth. */
  pp?: number;
  /** Bench steps: requested generation tokens. */
  tg?: number;
}

export interface TestDef {
  id: string;
  title: string;
  description?: string;
  temperature?: number | null;
  maxTokens?: number | null;
  /** "Treat LLM sessions as regimes": report splits by Section titles. */
  regimesFromSections?: boolean;
  prebuilt?: boolean;
  /** User-marked favorite; offered in the top-bar favorites dropdown. */
  favorite?: boolean;
  createdAt?: string;
  steps: TestStep[];
}
