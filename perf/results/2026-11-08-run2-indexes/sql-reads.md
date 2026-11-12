| Call | Logical reads (8 KB pages) | Reads per table | CPU ms | Elapsed ms |
|---|---:|---|---:|---:|
| client_search: newest first (MANAGER, 5M rows) | 53 | users 2, users 46, clients 5 | 4 | 2 |
| client_search: name prefix Zielony (MANAGER) | 157 | users 2, users 50, clients 105 | 4 | 2 |
| client_search: name prefix Zielony (SALES, own clients) | 115 | users 2, users 34, clients 79 | 2 | 2 |
| client_search: city + status (MANAGER) | 460 | users 2, users 50, clients 408 | 5 | 3 |
| client_search: exact NIP (MANAGER) | 11 | users 2, users 2, clients 7 | 4 | 2 |
| client_get | 12 | users 2, clients 4, users 2, clients 4 | 0 | 0 |
| project_search: all by due date (MANAGER, 5M rows) | 260 | users 2, teams 42, users 50, clients 92, projects 74 | 6 | 5 |
| project_search: overdue (MANAGER, filtered index) | 263 | users 2, teams 42, users 50, clients 92, projects 77 | 6 | 6 |
| project_search: stage ACTIVE (MANAGER) | 261 | users 2, teams 42, users 50, clients 92, projects 75 | 8 | 6 |
| project_search: team (MANAGER) | 220 | users 2, users 50, clients 92, projects 74, teams 2 | 5 | 5 |
| project_search: visible to SALES (own + team + clients) | 6049 | users 2, #B6C920E0 2220, projects 3574, clients 11, team_members 2, teams 42, users 43, clients 85, projects 64, #B6C920E0 6 | 13 | 13 |
| project_search: overdue visible to SALES | 4903 | users 2, #B6C920E0 1074, projects 3573, clients 11, team_members 2, teams 42, users 44, clients 86, projects 65, #B6C920E0 4 | 13 | 13 |
| project_get | 16 | users 2, projects 3, clients 4, users 2, teams 2, projects 3 | 0 | 0 |
| project_stage_history_list | 8 | users 2, projects 3, project_stage_history 3 | 2 | 0 |
| project_comment_list | 10 | users 2, projects 3, users 2, project_comments 3 | 0 | 0 |
| team_project_stats (indexed view, NOEXPAND) | 6 | users 2, teams 2, v_team_project_stats 2 | 0 | 0 |
| COMPARISON team stats counted from crm.projects (what the indexed view saves) | 16867 | projects 16867 | 7 | 7 |
| COMPARISON OFFSET paging, page 200 000 of clients (what keyset paging avoids) | 96730 | clients 96730 | 239 | 239 |
| COMPARISON keyset paging, the same page | 61 | clients 4, users 2, users 50, clients 5 | 3 | 3 |
