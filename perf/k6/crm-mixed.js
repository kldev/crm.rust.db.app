// Mixed CRM traffic on the seeded data set (ADR-0004): 200 virtual users by default.
// Each VU logs in once as its own seeded user (90% perf.sales.NNNN, 10% perf.manager.NNNN) and then repeats
// one weighted action per iteration: searches and details dominate, writes are about a quarter of iterations.
//
// Env: BASE_URL, CRM_SEED_PASSWORD, RESULTS_DIR, VUS (200), RAMP (2m), HOLD (10m), RAMP_DOWN (1m),
//      SALES_USERS (4500), MANAGER_USERS (490).
import http from 'k6/http';
import { check, sleep } from 'k6';
import { Trend, Counter } from 'k6/metrics';
import { textSummary } from 'https://jslib.k6.io/k6-summary/0.1.0/index.js';

const BASE = __ENV.BASE_URL || 'http://localhost:8088';
const PASSWORD = __ENV.CRM_SEED_PASSWORD;
const VUS = parseInt(__ENV.VUS || '200', 10);
const SALES_USERS = parseInt(__ENV.SALES_USERS || '4500', 10);
const MANAGER_USERS = parseInt(__ENV.MANAGER_USERS || '490', 10);

export const options = {
    // Keep the session cookie between iterations: each VU logs in once.
    noCookiesReset: true,
    scenarios: {
        mixed: {
            executor: 'ramping-vus',
            startVUs: 0,
            stages: [
                { duration: __ENV.RAMP || '2m', target: VUS },
                { duration: __ENV.HOLD || '10m', target: VUS },
                { duration: __ENV.RAMP_DOWN || '1m', target: 0 },
            ],
            gracefulRampDown: '30s',
        },
    },
    thresholds: {
        http_req_duration: ['p(95)<500', 'p(99)<1000'],
        http_req_failed: ['rate<0.01'],
    },
    summaryTrendStats: ['avg', 'min', 'med', 'p(90)', 'p(95)', 'p(99)', 'max'],
};

// One latency trend per endpoint, so the summary shows each of them.
const ENDPOINTS = [
    'login', 'clients_search', 'clients_search_next', 'client_get', 'client_create', 'client_update',
    'projects_search', 'project_get', 'project_stages', 'project_comments', 'comment_add', 'project_stage',
    'project_create', 'team_stats',
];
const trends = {};
for (const e of ENDPOINTS) trends[e] = new Trend(`ep_${e}`, true);
const unexpected = new Counter('unexpected_status');

const PREFIXES = ['Zielony', 'Szybki', 'Nowy', 'Stary', 'Wielki', 'Mały', 'Jasny', 'Złoty', 'Polski', 'Górski',
    'Morski', 'Leśny', 'Cichy', 'Mocny', 'Dobry', 'Mądry', 'Zielony M', 'Nowy Dom', 'Stary Most', 'Złoty Ró'];
const CITIES = ['Warszawa', 'Kraków', 'Łódź', 'Wrocław', 'Poznań', 'Gdańsk', 'Szczecin', 'Lublin', 'Rzeszów', 'Olsztyn'];
const STAGES = ['NEW', 'ACCEPTED', 'ACTIVE', 'ON_HOLD', 'CLOSED', 'DECLINED'];
const NEXT_STAGE = { NEW: ['ACCEPTED', 'DECLINED'], ACCEPTED: ['ACTIVE'], ACTIVE: ['ON_HOLD', 'CLOSED'], ON_HOLD: ['ACTIVE'] };

const pick = (list) => list[Math.floor(Math.random() * list.length)];

// Per-VU state (each VU is its own JS runtime).
const state = { me: null, xsrf: null, clients: [], projects: [] };

function call(name, method, path, body) {
    const params = {
        headers: { 'Content-Type': 'application/json', 'X-XSRF-TOKEN': state.xsrf || '' },
        tags: { name },
    };
    const res = http.request(method, `${BASE}${path}`, body ? JSON.stringify(body) : null, params);
    trends[name].add(res.timings.duration);
    if (res.status >= 400) {
        unexpected.add(1, { name, status: String(res.status) });
    }
    return res;
}

function xsrfFromJar() {
    const cookies = http.cookieJar().cookiesForURL(BASE);
    return cookies['XSRF-TOKEN'] ? cookies['XSRF-TOKEN'][0] : null;
}

function login() {
    const vu = __VU;
    const login = vu % 10 === 0
        ? `perf.manager.${String(((vu / 10) % MANAGER_USERS) + 1).padStart(4, '0')}`
        : `perf.sales.${String((vu % SALES_USERS) + 1).padStart(4, '0')}`;
    http.get(`${BASE}/api/auth/csrf`, { tags: { name: 'csrf' } });
    state.xsrf = xsrfFromJar();
    const res = call('login', 'POST', '/api/auth/login', { login, password: PASSWORD });
    check(res, { 'login 200': (r) => r.status === 200 });
    if (res.status === 200) {
        state.me = res.json();
        state.xsrf = xsrfFromJar();
    }
}

function rememberClients(res) {
    if (res.status !== 200) return null;
    const page = res.json();
    if (page.items.length) state.clients = page.items.slice(0, 10).map((c) => c.id);
    return page;
}

function rememberProjects(res) {
    if (res.status !== 200) return null;
    const page = res.json();
    if (page.items.length) state.projects = page.items.slice(0, 10);
    return page;
}

function searchClients() {
    const r = Math.random();
    let q;
    if (r < 0.5) q = `namePrefix=${encodeURIComponent(pick(PREFIXES))}`;
    else if (r < 0.75) q = `city=${encodeURIComponent(pick(CITIES))}&status=ACTIVE`;
    else q = '';
    const res = call('clients_search', 'GET', `/api/clients?${q}&size=20`);
    check(res, { 'clients 200': (x) => x.status === 200 });
    const page = rememberClients(res);
    if (page && page.nextCursor && Math.random() < 0.3) {
        call('clients_search_next', 'GET', `/api/clients?${q}&size=20&cursor=${page.nextCursor}`);
    }
}

function getClient() {
    if (!state.clients.length) return searchClients();
    const res = call('client_get', 'GET', `/api/clients/${pick(state.clients)}`);
    check(res, { 'client 200': (x) => x.status === 200 });
}

function updateClient() {
    if (!state.clients.length) return searchClients();
    const got = call('client_get', 'GET', `/api/clients/${pick(state.clients)}`);
    if (got.status !== 200) return;
    const c = got.json();
    const res = call('client_update', 'PUT', `/api/clients/${c.id}`, {
        name: c.name, nip: c.nip, email: c.email, phone: c.phone, city: c.city, status: c.status,
        assignedUserId: c.assignedUserId, note: `k6 note ${Date.now()}`, rowVersion: c.rowVersion,
    });
    check(res, { 'client update 200': (x) => x.status === 200 });
}

function createClient() {
    const res = call('client_create', 'POST', '/api/clients', {
        name: `K6 Klient ${__VU}-${__ITER}-${Date.now()}`, email: `k6.${__VU}.${__ITER}@example.test`,
        city: pick(CITIES), status: 'LEAD', // SALES: assigned to themselves; MANAGER: left unassigned
    });
    check(res, { 'client create 201': (x) => x.status === 201 });
}

function searchProjects() {
    const r = Math.random();
    let q;
    if (r < 0.3) q = 'overdue=true';
    else if (r < 0.6) q = `stage=${pick(STAGES)}`;
    else if (r < 0.8 && state.projects.length) q = `teamId=${pick(state.projects).teamId}`;
    else if (state.clients.length) q = `clientId=${pick(state.clients)}`;
    else q = '';
    const res = call('projects_search', 'GET', `/api/projects?${q}&size=20`);
    check(res, { 'projects 200': (x) => x.status === 200 });
    rememberProjects(res);
}

function projectDetails() {
    if (!state.projects.length) return searchProjects();
    const p = pick(state.projects);
    const res = call('project_get', 'GET', `/api/projects/${p.id}`);
    check(res, { 'project 200': (x) => x.status === 200 });
    call('project_stages', 'GET', `/api/projects/${p.id}/stages`);
}

function projectComments() {
    if (!state.projects.length) return searchProjects();
    const res = call('project_comments', 'GET', `/api/projects/${pick(state.projects).id}/comments?size=20`);
    check(res, { 'comments 200': (x) => x.status === 200 });
}

function addComment() {
    if (!state.projects.length) return searchProjects();
    const res = call('comment_add', 'POST', `/api/projects/${pick(state.projects).id}/comments`,
        { body: `k6 comment from VU ${__VU}, iteration ${__ITER}` });
    check(res, { 'comment 201': (x) => x.status === 201 });
}

function changeStage() {
    if (!state.projects.length) return searchProjects();
    const got = call('project_get', 'GET', `/api/projects/${pick(state.projects).id}`);
    if (got.status !== 200) return;
    const p = got.json();
    const next = NEXT_STAGE[p.stage];
    if (!next) return; // CLOSED / DECLINED are final
    const res = call('project_stage', 'POST', `/api/projects/${p.id}/stage`,
        { stage: pick(next), comment: Math.random() < 0.5 ? 'k6 stage change' : null, rowVersion: p.rowVersion });
    check(res, { 'stage 200': (x) => x.status === 200 });
}

function createProject() {
    // Own project: a SALES user is a member of the team of the projects they own.
    const own = state.projects.filter((p) => p.ownerUserId === state.me.id);
    if (!own.length) {
        const res = call('projects_search', 'GET', `/api/projects?ownerUserId=${state.me.id}&size=20`);
        rememberProjects(res);
        return;
    }
    const p = pick(own);
    const res = call('project_create', 'POST', '/api/projects', {
        clientId: p.clientId, teamId: p.teamId, ownerUserId: p.ownerUserId,
        name: `K6 projekt ${__VU}-${__ITER}`, startDate: '2026-11-01', dueDate: '2027-03-31', budget: 25000.5,
    });
    check(res, { 'project create 201': (x) => x.status === 201 });
}

function teamStats() {
    if (!state.projects.length) return searchProjects();
    const res = call('team_stats', 'GET', `/api/teams/${pick(state.projects).teamId}/stats`);
    check(res, { 'stats 200': (x) => x.status === 200 });
}

// Weights in percent.
const ACTIONS = [
    [25, searchClients], [10, getClient], [20, searchProjects], [10, projectDetails], [8, projectComments],
    [5, addComment], [5, changeStage], [5, updateClient], [4, createClient], [3, createProject], [5, teamStats],
];
const TOTAL = ACTIONS.reduce((s, a) => s + a[0], 0);

export default function () {
    if (!state.me) {
        login();
        if (!state.me) {
            sleep(1);
            return;
        }
    }
    let roll = Math.random() * TOTAL;
    for (const [weight, action] of ACTIONS) {
        roll -= weight;
        if (roll < 0) {
            action();
            break;
        }
    }
    sleep(0.3 + Math.random() * 0.7);
}

export function handleSummary(data) {
    const dir = __ENV.RESULTS_DIR || '.';
    return {
        stdout: textSummary(data, { indent: ' ', enableColors: false }),
        [`${dir}/summary.json`]: JSON.stringify(data, null, 2),
        [`${dir}/summary.txt`]: textSummary(data, { indent: ' ', enableColors: false }),
    };
}
