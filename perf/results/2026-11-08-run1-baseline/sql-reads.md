| Call | Logical reads (8 KB pages) | Reads per table | CPU ms | Elapsed ms |
|---|---:|---|---:|---:|
| client_search: newest first (MANAGER, 5M rows) | 53 | users 2, users 46, clients 5 | 2 | 2 |
| client_search: name prefix Zielony (MANAGER) | 157 | users 2, users 50, clients 105 | 3 | 3 |
| client_search: name prefix Zielony (SALES, own clients) | 4541 | users 2, users 34, clients 4505 | 7 | 5 |
| client_search: city + status (MANAGER) | 460 | users 2, users 50, clients 408 | 6 | 5 |
| client_search: exact NIP (MANAGER) | 11 | users 2, users 2, clients 7 | 3 | 2 |
| client_get | 12 | users 2, clients 4, users 2, clients 4 | 0 | 0 |
| project_search: all by due date (MANAGER, 5M rows) | 102267 | users 2, users 50, teams 42, clients 221, projects 101952 | 8627 | 8628 |
| project_search: overdue (MANAGER, filtered index) | 469 | users 2, teams 42, users 50, clients 223, projects 152 | 9 | 7 |
| project_search: stage ACTIVE (MANAGER) | 261 | users 2, teams 42, users 50, clients 92, projects 75 | 6 | 6 |
| project_search: team (MANAGER) | 424 | users 2, users 50, clients 221, projects 149, teams 2 | 7 | 5 |
| project_search: visible to SALES (own + team + clients) | 21671 | users 2, #AF27FF18 1009, projects 3646, clients 11, team_members 2, users 50, teams 42, clients 92, projects 16803, #AF27FF18 14 | 49 | 48 |
| project_search: overdue visible to SALES | 21671 | users 2, #AF27FF18 1009, projects 3646, clients 11, team_members 2, users 50, teams 42, clients 92, projects 16803, #AF27FF18 14 | 50 | 48 |
| project_get | 16 | users 2, projects 3, clients 4, users 2, teams 2, projects 3 | 0 | 0 |
| project_stage_history_list | 8 | users 2, projects 3, project_stage_history 3 | 0 | 0 |
| project_comment_list | 10 | users 2, projects 3, users 2, project_comments 3 | 0 | 0 |
| team_project_stats (indexed view, NOEXPAND) | 6 | users 2, teams 2, v_team_project_stats 2 | 0 | 0 |
| COMPARISON team stats counted from crm.projects (what the indexed view saves) | 16867 | projects 16867 | 6 | 6 |
| COMPARISON OFFSET paging, page 200 000 of clients (what keyset paging avoids) | 96940 | clients 96940 | 398 | 471 |
| COMPARISON keyset paging, the same page | 61 | clients 4, users 2, users 50, clients 5 | 3 | 3 |
