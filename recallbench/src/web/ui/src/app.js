let currentQuestions = [];

// Theme management
function setTheme(theme) {
    document.documentElement.setAttribute("data-theme", theme);
    localStorage.setItem("recallbench-theme", theme);
}

// Restore saved theme
const savedTheme = localStorage.getItem("recallbench-theme");
if (savedTheme) {
    document.documentElement.setAttribute("data-theme", savedTheme);
    const sel = document.getElementById("theme-select");
    if (sel) sel.value = savedTheme;
}

// Dashboard
async function showDashboard() {
    document.getElementById("dashboard").classList.remove("hidden");
    document.getElementById("run-detail").classList.add("hidden");

    const resp = await fetch("/api/runs");
    const runs = await resp.json();
    const container = document.getElementById("runs-list");

    if (runs.length === 0) {
        container.innerHTML = `
            <div class="card bg-base-200 col-span-full">
                <div class="card-body items-center text-center py-12">
                    <p class="text-base-content/50">No benchmark runs found.</p>
                    <p class="text-sm text-base-content/30">Run <code class="kbd kbd-sm">recallbench run</code> to generate results.</p>
                </div>
            </div>`;
        return;
    }

    container.innerHTML = runs.map(r => {
        const accClass = r.accuracy >= 0.9 ? "accuracy-good" : r.accuracy >= 0.7 ? "accuracy-mid" : "accuracy-bad";
        return `
            <div class="card bg-base-200 shadow-md hover:shadow-lg hover:border-primary border border-base-300 cursor-pointer transition-all" onclick="showRun('${r.id}')">
                <div class="card-body p-4">
                    <h3 class="card-title text-base">${r.system || "Unknown"}</h3>
                    <p class="text-3xl font-bold ${accClass}">${(r.accuracy * 100).toFixed(1)}%</p>
                    <p class="text-xs text-base-content/50">${r.total_questions} questions &middot; ${r.filename}</p>
                </div>
            </div>`;
    }).join("");
}

// Run Detail
async function showRun(id) {
    document.getElementById("dashboard").classList.add("hidden");
    document.getElementById("run-detail").classList.remove("hidden");
    document.getElementById("run-title").textContent = id;
    document.getElementById("run-breadcrumb").textContent = id;

    const [metricsResp, questionsResp] = await Promise.all([
        fetch(`/api/runs/${id}/metrics`),
        fetch(`/api/runs/${id}/questions`),
    ]);
    const metrics = await metricsResp.json();
    const questions = await questionsResp.json();
    currentQuestions = questions;

    const acc = metrics.accuracy || {};
    const lat = metrics.latency || {};
    const cost = metrics.cost || {};

    // Stats cards
    const accClass = (acc.overall || 0) >= 0.9 ? "text-success" : (acc.overall || 0) >= 0.7 ? "text-warning" : "text-error";
    document.getElementById("stats-cards").innerHTML = `
        <div class="stat bg-base-200 rounded-box p-3">
            <div class="stat-title text-xs">Task-Averaged</div>
            <div class="stat-value text-lg ${accClass}">${((acc.task_averaged || 0) * 100).toFixed(1)}%</div>
        </div>
        <div class="stat bg-base-200 rounded-box p-3">
            <div class="stat-title text-xs">Overall</div>
            <div class="stat-value text-lg ${accClass}">${((acc.overall || 0) * 100).toFixed(1)}%</div>
        </div>
        <div class="stat bg-base-200 rounded-box p-3">
            <div class="stat-title text-xs">Questions</div>
            <div class="stat-value text-lg">${acc.total_questions || 0}</div>
        </div>
        <div class="stat bg-base-200 rounded-box p-3">
            <div class="stat-title text-xs">Correct</div>
            <div class="stat-value text-lg text-success">${acc.total_correct || 0}</div>
        </div>
        <div class="stat bg-base-200 rounded-box p-3">
            <div class="stat-title text-xs">Retrieval p50</div>
            <div class="stat-value text-lg">${(lat.retrieval_p50 || 0).toFixed(0)}ms</div>
        </div>
        <div class="stat bg-base-200 rounded-box p-3">
            <div class="stat-title text-xs">Est. Cost</div>
            <div class="stat-value text-lg">$${(cost.estimated_usd || 0).toFixed(2)}</div>
        </div>
    `;

    // Per-type table
    if (acc.per_type) {
        const entries = Object.entries(acc.per_type).sort((a, b) => a[0].localeCompare(b[0]));
        document.getElementById("type-table-container").innerHTML = `
            <div class="overflow-x-auto">
                <table class="table table-sm table-zebra">
                    <thead><tr><th>Question Type</th><th>Accuracy</th></tr></thead>
                    <tbody>
                        ${entries.map(([t, v]) => {
                            const cls = v >= 0.9 ? "badge-success" : v >= 0.7 ? "badge-warning" : "badge-error";
                            return `<tr><td>${t}</td><td><span class="badge ${cls} badge-sm">${(v * 100).toFixed(1)}%</span></td></tr>`;
                        }).join("")}
                    </tbody>
                </table>
            </div>`;
    }

    // Populate type filter
    const types = [...new Set(questions.map(q => q.question_type))].sort();
    const sel = document.getElementById("type-filter");
    sel.innerHTML = '<option value="">All types</option>' + types.map(t => `<option value="${t}">${t}</option>`).join("");

    filterQuestions();
}

function filterQuestions() {
    const failOnly = document.getElementById("show-failures-only").checked;
    const typeFilter = document.getElementById("type-filter").value;
    let filtered = currentQuestions;
    if (failOnly) filtered = filtered.filter(q => !q.is_correct);
    if (typeFilter) filtered = filtered.filter(q => q.question_type === typeFilter);

    const tbody = document.getElementById("questions-body");
    tbody.innerHTML = filtered.map(q => `
        <tr>
            <td class="font-mono text-xs">${q.question_id}</td>
            <td><span class="badge badge-ghost badge-sm">${q.question_type}</span></td>
            <td>${q.is_correct
                ? '<span class="badge badge-success badge-sm">Pass</span>'
                : '<span class="badge badge-error badge-sm">Fail</span>'}</td>
            <td class="max-w-xs truncate">${q.ground_truth || ""}</td>
            <td class="max-w-xs truncate">${q.hypothesis || ""}</td>
        </tr>
    `).join("");
}

function truncate(s, n) { return s && s.length > n ? s.slice(0, n) + "..." : (s || ""); }

// Expose to window for onclick handlers in dynamic HTML
window.showDashboard = showDashboard;
window.showRun = showRun;
window.filterQuestions = filterQuestions;
window.setTheme = setTheme;

// Event listeners
document.getElementById("nav-dashboard")?.addEventListener("click", showDashboard);
document.querySelectorAll(".nav-home").forEach(el => el.addEventListener("click", showDashboard));
document.getElementById("theme-select")?.addEventListener("change", (e) => setTheme(e.target.value));
document.querySelectorAll(".filter-change").forEach(el => {
    el.addEventListener("change", filterQuestions);
});

// Initialize
showDashboard();
