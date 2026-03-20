let currentQuestions = [];

async function showDashboard() {
    document.getElementById('dashboard').style.display = '';
    document.getElementById('run-detail').style.display = 'none';
    const resp = await fetch('/api/runs');
    const runs = await resp.json();
    const container = document.getElementById('runs-list');
    if (runs.length === 0) {
        container.innerHTML = '<p style="color:#8b949e">No benchmark runs found. Run <code>recallbench run</code> to generate results.</p>';
        return;
    }
    container.innerHTML = runs.map(r => `
        <div class="card" onclick="showRun('${r.id}')">
            <h3>${r.system || 'Unknown'}</h3>
            <div class="accuracy">${(r.accuracy * 100).toFixed(1)}%</div>
            <div class="meta">${r.total_questions} questions &middot; ${r.filename}</div>
        </div>
    `).join('');
}

async function showRun(id) {
    document.getElementById('dashboard').style.display = 'none';
    document.getElementById('run-detail').style.display = '';
    document.getElementById('run-title').textContent = id;

    const [metricsResp, questionsResp] = await Promise.all([
        fetch(`/api/runs/${id}/metrics`),
        fetch(`/api/runs/${id}/questions`),
    ]);
    const metrics = await metricsResp.json();
    const questions = await questionsResp.json();
    currentQuestions = questions;

    const ms = document.getElementById('metrics-section');
    const acc = metrics.accuracy || {};
    const lat = metrics.latency || {};
    const cost = metrics.cost || {};

    ms.innerHTML = `
        <div class="metrics-grid">
            <div class="metric-card"><div class="label">Task-Averaged</div><div class="value">${((acc.task_averaged||0)*100).toFixed(1)}%</div></div>
            <div class="metric-card"><div class="label">Overall</div><div class="value">${((acc.overall||0)*100).toFixed(1)}%</div></div>
            <div class="metric-card"><div class="label">Questions</div><div class="value">${acc.total_questions||0}</div></div>
            <div class="metric-card"><div class="label">Correct</div><div class="value">${acc.total_correct||0}</div></div>
            <div class="metric-card"><div class="label">Retrieval p50</div><div class="value">${(lat.retrieval_p50||0).toFixed(0)}ms</div></div>
            <div class="metric-card"><div class="label">Est. Cost</div><div class="value">$${(cost.estimated_usd||0).toFixed(2)}</div></div>
        </div>
        ${acc.per_type ? renderPerType(acc.per_type) : ''}
    `;

    // Populate type filter
    const types = [...new Set(questions.map(q => q.question_type))].sort();
    const sel = document.getElementById('type-filter');
    sel.innerHTML = '<option value="">All types</option>' + types.map(t => `<option value="${t}">${t}</option>`).join('');

    filterQuestions();
}

function renderPerType(perType) {
    const entries = Object.entries(perType).sort((a,b) => a[0].localeCompare(b[0]));
    return `<table><thead><tr><th>Type</th><th>Accuracy</th></tr></thead><tbody>
        ${entries.map(([t,v]) => `<tr><td>${t}</td><td>${(v*100).toFixed(1)}%</td></tr>`).join('')}
    </tbody></table>`;
}

function filterQuestions() {
    const failOnly = document.getElementById('show-failures-only').checked;
    const typeFilter = document.getElementById('type-filter').value;
    let filtered = currentQuestions;
    if (failOnly) filtered = filtered.filter(q => !q.is_correct);
    if (typeFilter) filtered = filtered.filter(q => q.question_type === typeFilter);

    const tbody = document.getElementById('questions-body');
    tbody.innerHTML = filtered.map(q => `
        <tr>
            <td>${q.question_id}</td>
            <td>${q.question_type}</td>
            <td class="${q.is_correct ? 'correct' : 'incorrect'}">${q.is_correct ? 'Yes' : 'No'}</td>
            <td>${truncate(q.ground_truth, 60)}</td>
            <td>${truncate(q.hypothesis, 60)}</td>
        </tr>
    `).join('');
}

function truncate(s, n) { return s && s.length > n ? s.slice(0, n) + '...' : (s || ''); }

showDashboard();
