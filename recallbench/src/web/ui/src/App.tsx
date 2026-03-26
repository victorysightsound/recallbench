import { createSignal, createResource, For, Show, Component, onCleanup } from "solid-js";
import { createStore, reconcile } from "solid-js/store";
import { marked } from "marked";

interface RunSummary {
  id: string;
  filename: string;
  system: string | null;
  total_questions: number;
  accuracy: number;
  task_averaged: number;
  per_type: Record<string, [number, number]>;  // [correct, total]
  total_correct: number;
  estimated_cost: number;
  tokens_in: number;
  modified: string;
  total_target: number | null;
  dataset: string | null;
  variant: string | null;
  started_at: string | null;
  note: string | null;
  gen_model: string | null;
  judge_model: string | null;
}

interface Metrics {
  accuracy: {
    task_averaged: number;
    overall: number;
    per_type: Record<string, number>;
    abstention: number | null;
    total_questions: number;
    total_correct: number;
  };
  latency: { retrieval_p50: number; total_p50: number };
  cost: { tokens_in: number; tokens_out: number; estimated_usd: number };
}

interface Question {
  question_id: string;
  question_type: string;
  is_correct: boolean;
  ground_truth: string;
  hypothesis: string;
  is_abstention: boolean;
}

// Theme management
function setTheme(theme: string) {
  document.documentElement.setAttribute("data-theme", theme);
  localStorage.setItem("recallbench-theme", theme);
}

const savedTheme = localStorage.getItem("recallbench-theme") || "midnight";
document.documentElement.setAttribute("data-theme", savedTheme);

const THEMES = ["midnight", "dark", "light", "cyber", "onyx", "slate", "frost"];

// API fetchers
const fetchRuns = async (): Promise<RunSummary[]> => {
  const resp = await fetch("/api/runs");
  return resp.json();
};

const fetchMetrics = async (id: string): Promise<Metrics> => {
  const resp = await fetch(`/api/runs/${id}/metrics`);
  return resp.json();
};

const fetchQuestions = async (id: string): Promise<Question[]> => {
  const resp = await fetch(`/api/runs/${id}/questions`);
  return resp.json();
};

// Components

const ThemeSwitcher: Component = () => {
  const [theme, setThemeSignal] = createSignal(savedTheme);

  const onChange = (e: Event) => {
    const val = (e.target as HTMLSelectElement).value;
    setThemeSignal(val);
    setTheme(val);
  };

  return (
    <select class="select select-sm select-bordered w-28" value={theme()} onChange={onChange}>
      <For each={THEMES}>
        {(t) => <option value={t}>{t.charAt(0).toUpperCase() + t.slice(1)}</option>}
      </For>
    </select>
  );
};

const AccuracyBadge: Component<{ value: number; size?: string }> = (props) => {
  const cls = () =>
    props.value >= 0.9 ? "badge-success" : props.value >= 0.7 ? "badge-warning" : "badge-error";
  return (
    <span class={`badge ${cls()} ${props.size || "badge-sm"}`}>
      {(props.value * 100).toFixed(1)}%
    </span>
  );
};

/** Strip markdown formatting symbols */
const stripMd = (s: string) =>
  s.replace(/\*\*/g, "").replace(/\*/g, "").replace(/#{1,6}\s?/g, "").replace(/`/g, "").replace(/\n/g, " ").trim();

/** Format elapsed time as human readable */
const formatElapsed = (ms: number) => {
  const s = Math.floor(ms / 1000);
  if (s < 60) return `${s}s`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m}m ${s % 60}s`;
  const h = Math.floor(m / 60);
  return `${h}h ${m % 60}m`;
};

/** Render markdown string as formatted HTML */
const Markdown: Component<{ text: string }> = (props) => {
  const html = () => marked.parse(props.text || "", { async: false }) as string;
  return <div class="prose prose-sm max-w-none" innerHTML={html()} />;
};

const formatTokens = (n: number) =>
  n >= 1_000_000 ? `${(n / 1_000_000).toFixed(1)}M` : n >= 1_000 ? `${(n / 1_000).toFixed(0)}K` : `${n}`;

const formatDate = (iso: string) => {
  if (!iso) return "";
  const d = new Date(iso);
  return d.toLocaleDateString("en-US", { month: "short", day: "numeric", year: "numeric" }) +
    " " + d.toLocaleTimeString("en-US", { hour: "numeric", minute: "2-digit" });
};

/** Turn a filename like "femind-longmemeval-v3-verify.jsonl" into "Femind LongMemEval v3 Verify" */
const humanizeName = (filename: string) => {
  return filename
    .replace(/\.jsonl?$/, "")
    .split(/[-_]/)
    .map(w => w.charAt(0).toUpperCase() + w.slice(1))
    .join(" ");
};

const RunCard: Component<{ run: RunSummary; onClick: () => void; onExpand: () => void }> = (props) => {
  const [expanded, setExpanded] = createSignal(false);
  const accClass = () =>
    props.run.accuracy >= 0.9 ? "text-success" : props.run.accuracy >= 0.7 ? "text-warning" : "text-error";
  const types = () => Object.entries(props.run.per_type || {}).sort((a, b) => a[0].localeCompare(b[0]));

  const toggle = (e: MouseEvent) => {
    e.stopPropagation();
    setExpanded(!expanded());
  };

  return (
    <div
      class="card bg-base-200 shadow-md hover:shadow-lg hover:border-primary border border-base-300 cursor-pointer transition-all"
      onClick={props.onExpand}
    >
      <div class="card-body p-4 gap-2">
        {/* Header: name, date, expand toggle */}
        <div class="flex justify-between items-center">
          <div>
            <h3 class="font-semibold text-base">{humanizeName(props.run.filename)}</h3>
            <span class="text-xs text-base-content/40">{props.run.system} &middot; {formatDate(props.run.modified)}</span>
          </div>
          <div class="flex items-center gap-3">
            <div class={`text-2xl font-bold ${accClass()}`}>{(props.run.task_averaged * 100).toFixed(1)}%</div>
            <button class="btn btn-ghost btn-xs" onClick={toggle}>
              {expanded() ? "▲" : "▼"}
            </button>
          </div>
        </div>

        {/* Compact: progress bar */}
        <div class="flex items-center gap-2 text-xs text-base-content/50">
          <span>{props.run.total_correct}/{props.run.total_target || props.run.total_questions}</span>
          <div class="flex-1 bg-base-300 rounded-full h-1.5">
            <div
              class={`h-1.5 rounded-full transition-all ${props.run.accuracy >= 0.9 ? 'bg-success' : props.run.accuracy >= 0.7 ? 'bg-warning' : 'bg-error'}`}
              style={{ width: props.run.total_target ? `${Math.round((props.run.total_questions / props.run.total_target) * 100)}%` : "100%" }}
            ></div>
          </div>
          <span>{(props.run.accuracy * 100).toFixed(1)}%</span>
        </div>

        {/* Expanded: full details */}
        <Show when={expanded()}>
          <div class="divider my-1"></div>

          {/* Scores */}
          <div class="grid grid-cols-3 gap-3 text-center">
            <div>
              <div class="text-xs text-base-content/50 uppercase">Overall</div>
              <div class={`text-lg font-bold ${accClass()}`}>{(props.run.accuracy * 100).toFixed(1)}%</div>
            </div>
            <div>
              <div class="text-xs text-base-content/50 uppercase">Task-Avg</div>
              <div class={`text-lg font-bold ${accClass()}`}>{(props.run.task_averaged * 100).toFixed(1)}%</div>
            </div>
            <div>
              <div class="text-xs text-base-content/50 uppercase">Correct</div>
              <div class="text-lg font-bold">{props.run.total_correct}<span class="text-sm text-base-content/40">/{props.run.total_questions}</span></div>
            </div>
          </div>

          {/* Per-type breakdown */}
          <Show when={types().length > 0}>
            <div class="divider my-1"></div>
            <div class="space-y-1.5">
              <For each={types()}>
                {([type, [correct, total]]) => {
                  const pct = total > 0 ? correct / total : 0;
                  const cls = pct >= 0.95 ? "text-success" : pct >= 0.85 ? "text-warning" : "text-error";
                  return (
                    <div>
                      <div class="flex justify-between text-xs mb-0.5">
                        <span class="text-base-content/60">{type}</span>
                        <span class={cls}>{correct}/{total} ({(pct * 100).toFixed(0)}%)</span>
                      </div>
                      <div class="w-full bg-base-300 rounded-full h-1">
                        <div class={`h-1 rounded-full ${pct >= 0.95 ? 'bg-success' : pct >= 0.85 ? 'bg-warning' : 'bg-error'}`} style={{ width: `${Math.round(pct * 100)}%` }}></div>
                      </div>
                    </div>
                  );
                }}
              </For>
            </div>
          </Show>

          {/* Footer */}
          <div class="divider my-1"></div>
          <div class="flex justify-between text-xs text-base-content/40">
            <span>Tokens: {formatTokens(props.run.tokens_in)}</span>
            <span>Est. cost: ${props.run.estimated_cost.toFixed(2)}</span>
          </div>
        </Show>
      </div>
    </div>
  );
};

const StatCard: Component<{ label: string; value: string; class?: string }> = (props) => (
  <div class="stat bg-base-200 rounded-box p-3">
    <div class="stat-title text-xs">{props.label}</div>
    <div class={`stat-value text-lg ${props.class || ""}`}>{props.value}</div>
  </div>
);

const Dashboard: Component<{ onSelectRun: (id: string) => void }> = (props) => {
  const [state, setState] = createStore<{ runs: RunSummary[]; loaded: boolean }>({ runs: [], loaded: false });

  const refresh = async () => {
    const data = await fetchRuns();
    setState("runs", reconcile(data, { key: "id", merge: true }));
    setState("loaded", true);
  };
  refresh();
  const interval = setInterval(refresh, 5000);
  onCleanup(() => clearInterval(interval));

  return (
    <div>
      <h2 class="text-2xl font-bold mb-4">Benchmark Runs</h2>
      <Show when={state.loaded} fallback={<div class="skeleton h-32 w-full" />}>
        <Show
          when={state.runs.length > 0}
          fallback={
            <div class="card bg-base-200">
              <div class="card-body items-center text-center py-12">
                <p class="text-base-content/50">No benchmark runs found.</p>
                <p class="text-sm text-base-content/30">
                  Run <kbd class="kbd kbd-sm">recallbench run</kbd> to generate results.
                </p>
              </div>
            </div>
          }
        >
          <div class="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
            <For each={state.runs}>
              {(run) => <RunCard run={run} onClick={() => {}} onExpand={() => props.onSelectRun(run.id)} />}
            </For>
          </div>
        </Show>
      </Show>
    </div>
  );
};

const RunDetail: Component<{ runId: string; onBack: () => void }> = (props) => {
  const [store, setStore] = createStore<{ metrics: Metrics | null; questions: Question[] }>({
    metrics: null,
    questions: [],
  });
  const [failOnly, setFailOnly] = createSignal(false);
  const [typeFilter, setTypeFilter] = createSignal("");

  const [runInfo, setRunInfo] = createSignal<RunSummary | null>(null);

  const refresh = async () => {
    const [m, q, allRuns] = await Promise.all([
      fetchMetrics(props.runId),
      fetchQuestions(props.runId),
      fetchRuns(),
    ]);
    setStore("metrics", reconcile(m));
    setStore("questions", reconcile(q, { key: "question_id", merge: true }));
    const thisRun = allRuns.find((r: RunSummary) => r.id === props.runId);
    if (thisRun) setRunInfo(thisRun);
  };
  refresh();
  const interval = setInterval(refresh, 5000);
  onCleanup(() => clearInterval(interval));

  const metrics = () => store.metrics;
  const questions = () => store.questions;

  const filteredQuestions = () => {
    let qs = questions() || [];
    if (failOnly()) qs = qs.filter((q) => !q.is_correct);
    if (typeFilter()) qs = qs.filter((q) => q.question_type === typeFilter());
    return qs;
  };

  const questionTypes = () => {
    const types = new Set((questions() || []).map((q) => q.question_type));
    return [...types].sort();
  };

  const accClass = () => {
    const overall = metrics()?.accuracy?.overall || 0;
    return overall >= 0.9 ? "text-success" : overall >= 0.7 ? "text-warning" : "text-error";
  };

  const [questionsOpen, setQuestionsOpen] = createSignal(false);
  const [pageSize, setPageSize] = createSignal(10);
  const [page, setPage] = createSignal(0);
  const [now, setNow] = createSignal(Date.now());

  // Tick every second for elapsed/ETA display
  const ticker = setInterval(() => setNow(Date.now()), 1000);
  onCleanup(() => clearInterval(ticker));

  const pagedQuestions = () => {
    const filtered = filteredQuestions();
    const start = page() * pageSize();
    return filtered.slice(start, start + pageSize());
  };

  const totalPages = () => Math.ceil(filteredQuestions().length / pageSize());

  // Build per-type counts from questions signal for progress bars
  const perTypeCounts = () => {
    const counts: Record<string, { correct: number; total: number }> = {};
    for (const q of questions()) {
      if (!counts[q.question_type]) counts[q.question_type] = { correct: 0, total: 0 };
      counts[q.question_type].total++;
      if (q.is_correct) counts[q.question_type].correct++;
    }
    return Object.entries(counts).sort((a, b) => a[0].localeCompare(b[0]));
  };

  return (
    <div>
      {/* Breadcrumb */}
      <div class="breadcrumbs text-sm mb-4">
        <ul>
          <li><a class="cursor-pointer" onClick={props.onBack}>Dashboard</a></li>
          <li>{humanizeName(props.runId)}</li>
        </ul>
      </div>

      <h2 class="text-2xl font-bold mb-2">{humanizeName(props.runId)}</h2>

      {/* Run notes */}
      <Show when={runInfo()?.note || runInfo()?.gen_model}>
        <div class="bg-base-200 rounded-box p-4 mb-4 text-sm space-y-1">
          <Show when={runInfo()?.note}>
            <p class="text-base-content/80">{runInfo()!.note}</p>
          </Show>
          <div class="flex flex-wrap gap-3 text-xs text-base-content/50 mt-2">
            <Show when={runInfo()?.dataset}>
              <span>Dataset: <strong class="text-base-content/70">{runInfo()!.dataset} ({runInfo()!.variant})</strong></span>
            </Show>
            <Show when={runInfo()?.gen_model}>
              <span>Gen: <strong class="text-base-content/70">{runInfo()!.gen_model}</strong></span>
            </Show>
            <Show when={runInfo()?.judge_model}>
              <span>Judge: <strong class="text-base-content/70">{runInfo()!.judge_model}</strong></span>
            </Show>
            <Show when={runInfo()?.started_at}>
              <span>Run: <strong class="text-base-content/70">{formatDate(runInfo()!.started_at!)}</strong></span>
            </Show>
          </div>
        </div>
      </Show>

      {/* Overall progress bar */}
      <Show when={metrics()}>
        {(m) => {
          const evaluated = () => m().accuracy.total_questions;
          const target = () => runInfo()?.total_target || evaluated();
          const progressPct = () => target() > 0 ? evaluated() / target() : 1;
          const isRunning = () => target() > evaluated();
          const startedAt = () => runInfo()?.started_at;
          const elapsed = () => {
            const sa = startedAt();
            if (!sa) return 0;
            return now() - new Date(sa).getTime();
          };
          const msPerQ = () => evaluated() > 0 ? elapsed() / evaluated() : 0;
          const remaining = () => (target() - evaluated()) * msPerQ();
          return (
            <div class="mb-6">
              <div class="flex justify-between items-baseline mb-1">
                <span class="text-sm text-base-content/60">
                  Progress: {evaluated()}/{target()}
                  {isRunning() ? <span class="badge badge-info badge-xs ml-2">Running</span> : <span class="badge badge-success badge-xs ml-2">Complete</span>}
                </span>
                <span class="text-xs text-base-content/40">
                  {elapsed() > 0 ? `Elapsed: ${formatElapsed(elapsed())}` : ""}
                  {isRunning() && msPerQ() > 0 ? ` — ETA: ${formatElapsed(remaining())}` : ""}
                </span>
              </div>
              <div class="w-full bg-base-300 rounded-full h-2.5">
                <div
                  class={`h-2.5 rounded-full transition-all ${isRunning() ? 'bg-info' : 'bg-success'}`}
                  style={{ width: `${Math.round(progressPct() * 100)}%` }}
                ></div>
              </div>
            </div>
          );
        }}
      </Show>

      <Show when={metrics()}>
        {(m) => (
          <>
            {/* Stats Cards */}
            <div class="grid grid-cols-2 md:grid-cols-3 lg:grid-cols-6 gap-3 mb-6">
              <StatCard label="Task-Averaged" value={`${(m().accuracy.task_averaged * 100).toFixed(1)}%`} class={accClass()} />
              <StatCard label="Overall" value={`${(m().accuracy.overall * 100).toFixed(1)}%`} class={accClass()} />
              <StatCard label="Evaluated" value={`${m().accuracy.total_questions}`} />
              <StatCard label="Correct" value={`${m().accuracy.total_correct}`} class="text-success" />
              <StatCard label="Retrieval p50" value={`${m().latency.retrieval_p50.toFixed(0)}ms`} />
              <StatCard label="Est. Cost" value={`$${m().cost.estimated_usd.toFixed(2)}`} />
            </div>
          </>
        )}
      </Show>

      {/* Per-Type Breakdown with progress bars */}
      <Show when={perTypeCounts().length > 0}>
        <h3 class="text-lg font-semibold mb-3">Per-Type Accuracy</h3>
        <div class="space-y-2 mb-6">
          <For each={perTypeCounts()}>
            {([type, { correct, total }]) => {
              const pct = total > 0 ? correct / total : 0;
              const cls = pct >= 0.95 ? "text-success" : pct >= 0.85 ? "text-warning" : "text-error";
              return (
                <div>
                  <div class="flex justify-between text-sm mb-1">
                    <span>{type}</span>
                    <span class={cls}>{correct}/{total} ({(pct * 100).toFixed(1)}%)</span>
                  </div>
                  <div class="w-full bg-base-300 rounded-full h-2">
                    <div
                      class={`h-2 rounded-full ${pct >= 0.95 ? 'bg-success' : pct >= 0.85 ? 'bg-warning' : 'bg-error'}`}
                      style={{ width: `${Math.round(pct * 100)}%` }}
                    ></div>
                  </div>
                </div>
              );
            }}
          </For>
        </div>
      </Show>

      {/* Questions — collapsible */}
      <div
        class="flex items-center justify-between cursor-pointer py-2"
        onClick={() => setQuestionsOpen(!questionsOpen())}
      >
        <h3 class="text-lg font-semibold">
          Questions ({filteredQuestions().length})
          <span class="text-sm font-normal text-base-content/40 ml-2">{questionsOpen() ? "▲" : "▼"}</span>
        </h3>
        <Show when={questionsOpen()}>
          <div class="flex gap-3 items-center" onClick={(e: MouseEvent) => e.stopPropagation()}>
            <label class="label cursor-pointer gap-2">
              <span class="label-text text-sm">Failures only</span>
              <input
                type="checkbox"
                class="toggle toggle-sm toggle-error"
                checked={failOnly()}
                onChange={(e) => setFailOnly(e.target.checked)}
              />
            </label>
            <select
              class="select select-sm select-bordered"
              value={typeFilter()}
              onChange={(e) => setTypeFilter(e.target.value)}
            >
              <option value="">All types</option>
              <For each={questionTypes()}>
                {(t) => <option value={t}>{t}</option>}
              </For>
            </select>
            <select
              class="select select-sm select-bordered w-20"
              value={pageSize()}
              onChange={(e) => { setPageSize(parseInt(e.target.value)); setPage(0); }}
            >
              <option value="10">10</option>
              <option value="25">25</option>
              <option value="50">50</option>
              <option value="100">100</option>
              <option value="500">All</option>
            </select>
          </div>
        </Show>
      </div>

      <Show when={questionsOpen()}>
        <Show when={questions().length > 0} fallback={<div class="skeleton h-24 w-full" />}>
          <div class="overflow-y-auto max-h-[60vh] w-full border border-base-300 rounded-box">
            <table class="table table-zebra table-sm table-fixed w-full">
              <thead class="sticky top-0 bg-base-200 z-10">
                <tr>
                  <th class="w-[12%]">ID</th>
                  <th class="w-[14%]">Type</th>
                  <th class="w-[8%]">Correct</th>
                  <th class="w-[33%]">Ground Truth</th>
                  <th class="w-[33%]">Hypothesis</th>
                </tr>
              </thead>
              <tbody>
                <For each={pagedQuestions()}>
                  {(q) => {
                    const [showDetail, setShowDetail] = createSignal(false);
                    return (
                      <>
                        <tr class="cursor-pointer hover" onClick={() => setShowDetail(!showDetail())}>
                          <td class="font-mono text-xs">{q.question_id}</td>
                          <td><span class="badge badge-ghost badge-sm">{q.question_type}</span></td>
                          <td>
                            {q.is_correct
                              ? <span class="badge badge-success badge-sm">Pass</span>
                              : <span class="badge badge-error badge-sm">Fail</span>}
                          </td>
                          <td><div class="truncate text-left">{stripMd(q.ground_truth)}</div></td>
                          <td><div class="truncate text-left">{stripMd(q.hypothesis)}</div></td>
                        </tr>
                        <Show when={showDetail()}>
                          <tr>
                            <td colspan="5" class="bg-base-300 p-4">
                              <div class="space-y-3 text-sm max-w-full overflow-hidden">
                                <div>
                                  <span class="font-semibold text-base-content/70">Ground Truth:</span>
                                  <p class="mt-1 break-words">{q.ground_truth}</p>
                                </div>
                                <div class="divider my-1"></div>
                                <div>
                                  <span class="font-semibold text-base-content/70">Model Response:</span>
                                  <div class="mt-1 break-words overflow-hidden">
                                    <Markdown text={q.hypothesis} />
                                  </div>
                                </div>
                              </div>
                            </td>
                          </tr>
                        </Show>
                      </>
                    );
                  }}
                </For>
              </tbody>
            </table>
          </div>

          {/* Pagination */}
          <Show when={totalPages() > 1}>
            <div class="flex justify-center items-center gap-2 mt-4">
              <button
                class="btn btn-sm btn-ghost"
                disabled={page() === 0}
                onClick={() => setPage(page() - 1)}
              >Previous</button>
              <span class="text-sm text-base-content/60">
                Page {page() + 1} of {totalPages()}
              </span>
              <button
                class="btn btn-sm btn-ghost"
                disabled={page() >= totalPages() - 1}
                onClick={() => setPage(page() + 1)}
              >Next</button>
            </div>
          </Show>
        </Show>
      </Show>
    </div>
  );
};

// Comparison View
const CompareView: Component<{ onBack: () => void }> = (props) => {
  const [runs] = createResource(fetchRuns);
  const [selectedA, setSelectedA] = createSignal("");
  const [selectedB, setSelectedB] = createSignal("");
  const [metricsA] = createResource(() => selectedA() || undefined, (id) => fetchMetrics(id));
  const [metricsB] = createResource(() => selectedB() || undefined, (id) => fetchMetrics(id));

  return (
    <div>
      <div class="breadcrumbs text-sm mb-4">
        <ul><li><a class="cursor-pointer" onClick={props.onBack}>Dashboard</a></li><li>Compare</li></ul>
      </div>
      <h2 class="text-2xl font-bold mb-4">Compare Systems</h2>

      <div class="flex gap-4 mb-6">
        <select class="select select-bordered flex-1" onChange={(e) => setSelectedA(e.target.value)}>
          <option value="">Select System A</option>
          <Show when={runs()}><For each={runs()!}>{(r) => <option value={r.id}>{r.system} ({r.filename})</option>}</For></Show>
        </select>
        <select class="select select-bordered flex-1" onChange={(e) => setSelectedB(e.target.value)}>
          <option value="">Select System B</option>
          <Show when={runs()}><For each={runs()!}>{(r) => <option value={r.id}>{r.system} ({r.filename})</option>}</For></Show>
        </select>
      </div>

      <Show when={metricsA() && metricsB()}>
        <div class="overflow-x-auto">
          <table class="table table-zebra">
            <thead><tr><th>Metric</th><th>{selectedA()}</th><th>{selectedB()}</th></tr></thead>
            <tbody>
              <tr>
                <td class="font-semibold">Task-Averaged</td>
                <td><AccuracyBadge value={metricsA()!.accuracy.task_averaged} /></td>
                <td><AccuracyBadge value={metricsB()!.accuracy.task_averaged} /></td>
              </tr>
              <tr>
                <td class="font-semibold">Overall</td>
                <td><AccuracyBadge value={metricsA()!.accuracy.overall} /></td>
                <td><AccuracyBadge value={metricsB()!.accuracy.overall} /></td>
              </tr>
              <tr>
                <td class="font-semibold">Questions</td>
                <td>{metricsA()!.accuracy.total_questions}</td>
                <td>{metricsB()!.accuracy.total_questions}</td>
              </tr>
              <tr>
                <td class="font-semibold">Retrieval p50</td>
                <td>{metricsA()!.latency.retrieval_p50.toFixed(0)}ms</td>
                <td>{metricsB()!.latency.retrieval_p50.toFixed(0)}ms</td>
              </tr>
              <tr>
                <td class="font-semibold">Est. Cost</td>
                <td>${metricsA()!.cost.estimated_usd.toFixed(2)}</td>
                <td>${metricsB()!.cost.estimated_usd.toFixed(2)}</td>
              </tr>
            </tbody>
          </table>
        </div>

        {/* Per-type comparison */}
        <h3 class="text-lg font-semibold mt-6 mb-3">Per-Type Accuracy</h3>
        <div class="overflow-x-auto">
          <table class="table table-zebra table-sm">
            <thead><tr><th>Type</th><th>{selectedA()}</th><th>{selectedB()}</th><th>Diff</th></tr></thead>
            <tbody>
              <For each={Object.keys({...metricsA()!.accuracy.per_type, ...metricsB()!.accuracy.per_type}).sort()}>
                {(type) => {
                  const a = metricsA()!.accuracy.per_type[type] || 0;
                  const b = metricsB()!.accuracy.per_type[type] || 0;
                  const diff = a - b;
                  const diffCls = diff > 0 ? "text-success" : diff < 0 ? "text-error" : "";
                  return (
                    <tr>
                      <td>{type}</td>
                      <td><AccuracyBadge value={a} /></td>
                      <td><AccuracyBadge value={b} /></td>
                      <td class={diffCls}>{diff > 0 ? "+" : ""}{(diff * 100).toFixed(1)}%</td>
                    </tr>
                  );
                }}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </div>
  );
};

// Longevity View
const LongevityView: Component<{ onBack: () => void }> = (props) => {
  const [data, setData] = createSignal<any>(null);

  const loadFile = async (e: Event) => {
    const file = (e.target as HTMLInputElement).files?.[0];
    if (!file) return;
    const text = await file.text();
    setData(JSON.parse(text));
  };

  const maxAcc = () => data() ? Math.max(...data().checkpoints.map((c: any) => c.accuracy)) : 1;
  const maxLat = () => data() ? Math.max(...data().checkpoints.map((c: any) => c.avg_retrieval_latency_ms)) : 1;

  return (
    <div>
      <div class="breadcrumbs text-sm mb-4">
        <ul><li><a class="cursor-pointer" onClick={props.onBack}>Dashboard</a></li><li>Longevity</li></ul>
      </div>
      <h2 class="text-2xl font-bold mb-4">Longevity Analysis</h2>

      <div class="mb-6">
        <input type="file" accept=".json" class="file-input file-input-bordered" onChange={loadFile} />
        <p class="text-xs text-base-content/50 mt-1">Load a longevity result JSON file from <code>recallbench longevity --output</code></p>
      </div>

      <Show when={data()}>
        <h3 class="text-lg font-semibold mb-2">{data().system_name} — Accuracy Over Time</h3>
        <svg viewBox="0 0 600 200" class="w-full max-w-2xl mb-6 bg-base-200 rounded-box p-2">
          <For each={data().checkpoints}>
            {(cp: any, i) => {
              const x = () => 50 + (i() / (data().checkpoints.length - 1 || 1)) * 500;
              const y = () => 180 - (cp.accuracy / (maxAcc() || 1)) * 160;
              const nextCp = () => data().checkpoints[i() + 1];
              return (
                <>
                  <circle cx={x()} cy={y()} r="4" fill="oklch(76% 0.177 163.223)" />
                  <text x={x()} y={y() - 10} text-anchor="middle" font-size="10" fill="currentColor">
                    {(cp.accuracy * 100).toFixed(0)}%
                  </text>
                  <Show when={nextCp()}>
                    {(next) => {
                      const nx = () => 50 + ((i() + 1) / (data().checkpoints.length - 1 || 1)) * 500;
                      const ny = () => 180 - (next().accuracy / (maxAcc() || 1)) * 160;
                      return <line x1={x()} y1={y()} x2={nx()} y2={ny()} stroke="oklch(76% 0.177 163.223)" stroke-width="2" />;
                    }}
                  </Show>
                  <text x={x()} y={195} text-anchor="middle" font-size="9" fill="currentColor" opacity="0.5">
                    {cp.sessions_ingested}
                  </text>
                </>
              );
            }}
          </For>
          <text x="300" y="12" text-anchor="middle" font-size="11" fill="currentColor">Accuracy vs Sessions Ingested</text>
        </svg>

        <h3 class="text-lg font-semibold mb-2">Retrieval Latency Over Time</h3>
        <svg viewBox="0 0 600 200" class="w-full max-w-2xl mb-6 bg-base-200 rounded-box p-2">
          <For each={data().checkpoints}>
            {(cp: any, i) => {
              const x = () => 50 + (i() / (data().checkpoints.length - 1 || 1)) * 500;
              const y = () => 180 - (cp.avg_retrieval_latency_ms / (maxLat() || 1)) * 160;
              const nextCp = () => data().checkpoints[i() + 1];
              return (
                <>
                  <circle cx={x()} cy={y()} r="4" fill="oklch(74% 0.16 232.661)" />
                  <text x={x()} y={y() - 10} text-anchor="middle" font-size="10" fill="currentColor">
                    {cp.avg_retrieval_latency_ms.toFixed(0)}ms
                  </text>
                  <Show when={nextCp()}>
                    {(next) => {
                      const nx = () => 50 + ((i() + 1) / (data().checkpoints.length - 1 || 1)) * 500;
                      const ny = () => 180 - (next().avg_retrieval_latency_ms / (maxLat() || 1)) * 160;
                      return <line x1={x()} y1={y()} x2={nx()} y2={ny()} stroke="oklch(74% 0.16 232.661)" stroke-width="2" />;
                    }}
                  </Show>
                  <text x={x()} y={195} text-anchor="middle" font-size="9" fill="currentColor" opacity="0.5">
                    {cp.sessions_ingested}
                  </text>
                </>
              );
            }}
          </For>
          <text x="300" y="12" text-anchor="middle" font-size="11" fill="currentColor">Latency (ms) vs Sessions Ingested</text>
        </svg>

        {/* Data table */}
        <div class="overflow-x-auto">
          <table class="table table-zebra table-sm">
            <thead><tr><th>Sessions</th><th>Memories</th><th>Accuracy</th><th>Retrieval (ms)</th><th>Ingest (ms)</th></tr></thead>
            <tbody>
              <For each={data().checkpoints}>
                {(cp: any) => (
                  <tr>
                    <td>{cp.sessions_ingested}</td>
                    <td>{cp.estimated_memories}</td>
                    <td><AccuracyBadge value={cp.accuracy} /></td>
                    <td>{cp.avg_retrieval_latency_ms.toFixed(1)}</td>
                    <td>{cp.avg_ingest_latency_ms.toFixed(1)}</td>
                  </tr>
                )}
              </For>
            </tbody>
          </table>
        </div>
      </Show>
    </div>
  );
};

// Main App
const App: Component = () => {
  const [view, setViewRaw] = createSignal<"dashboard" | "detail" | "compare" | "longevity">("dashboard");
  const [selectedRun, setSelectedRun] = createSignal("");

  // Navigate with browser history support
  const navigate = (newView: typeof view extends () => infer T ? T : never, runId?: string) => {
    if (runId !== undefined) setSelectedRun(runId);
    setViewRaw(newView);
    const url = newView === "dashboard" ? "/" : newView === "detail" ? `/run/${runId || selectedRun()}` : `/${newView}`;
    window.history.pushState({ view: newView, runId: runId || selectedRun() }, "", url);
  };

  const showRun = (id: string) => navigate("detail", id);
  const showDashboard = () => navigate("dashboard", "");

  // Handle browser back/forward
  window.addEventListener("popstate", (e) => {
    const state = e.state;
    if (state?.view) {
      setViewRaw(state.view);
      if (state.runId) setSelectedRun(state.runId);
      else setSelectedRun("");
    } else {
      setViewRaw("dashboard");
      setSelectedRun("");
    }
  });

  // Set initial history state
  window.history.replaceState({ view: "dashboard", runId: "" }, "", "/");

  return (
    <div class="bg-base-100 text-base-content min-h-screen flex flex-col">
      {/* Navbar */}
      <div class="navbar bg-base-200 border-b border-base-300 px-4">
        <div class="navbar-start">
          <span class="text-primary font-bold text-lg cursor-pointer" onClick={showDashboard}>
            RecallBench
          </span>
          <span class="text-base-content/50 text-sm ml-2">Universal AI Memory Benchmark</span>
        </div>
        <div class="navbar-end gap-2">
          <button class="btn btn-sm btn-ghost" onClick={showDashboard}>Dashboard</button>
          <button class="btn btn-sm btn-ghost" onClick={() => navigate("compare")}>Compare</button>
          <button class="btn btn-sm btn-ghost" onClick={() => navigate("longevity")}>Longevity</button>
          <ThemeSwitcher />
        </div>
      </div>

      {/* Content */}
      <main class="flex-1 p-6 max-w-7xl mx-auto w-full">
        <Show when={view() === "dashboard"}>
          <Dashboard onSelectRun={showRun} />
        </Show>
        <Show when={view() === "detail"}>
          <RunDetail runId={selectedRun()} onBack={showDashboard} />
        </Show>
        <Show when={view() === "compare"}>
          <CompareView onBack={showDashboard} />
        </Show>
        <Show when={view() === "longevity"}>
          <LongevityView onBack={showDashboard} />
        </Show>
      </main>

      {/* Footer */}
      <footer class="footer footer-center p-3 bg-base-200 text-base-content/50 text-xs border-t border-base-300">
        <p>RecallBench — Universal AI Memory System Benchmark</p>
      </footer>
    </div>
  );
};

export default App;
