| Call | Shared buffers (8 KB blocks) | of which read from disk | Planning ms | Execution ms |
|---|---:|---:|---:|---:|
| client_search: newest first (MANAGER, 5M rows) | 84 | 0 | 0.01 | 0.12 |
| client_search: name prefix Zielony (MANAGER) | 104 | 0 | 0.01 | 0.15 |
| client_search: name prefix Zielony (SALES, own clients) | 27 | 0 | 0.00 | 0.09 |
| client_search: city + status (MANAGER) | 86 | 0 | 0.00 | 0.11 |
| client_search: exact NIP (MANAGER) | 23 | 0 | 0.00 | 0.08 |
| client_get | 10 | 0 | 0.00 | 0.05 |
| project_search: all by due date (MANAGER, 5M rows) | 272 | 0 | 0.01 | 0.43 |
| project_search: overdue (MANAGER, partial index) | 278 | 0 | 0.01 | 0.44 |
| project_search: stage ACTIVE (MANAGER) | 280 | 0 | 0.01 | 0.44 |
| project_search: team (MANAGER) | 156 | 0 | 0.01 | 0.32 |
| project_search: visible to SALES (own + team + clients) | 5898 | 0 | 0.01 | 2.14 |
| project_search: overdue visible to SALES | 5754 | 0 | 0.01 | 1.89 |
| project_get | 62 | 0 | 0.01 | 0.28 |
| project_stage_history_list | 11 | 0 | 0.00 | 0.11 |
| project_comment_list | 17 | 0 | 0.00 | 0.06 |
| team_project_stats (crm.team_project_stats) | 14 | 0 | 0.00 | 0.03 |
| COMPARISON team stats counted from crm.projects | 5380 | 0 | 0.05 | 2.23 |
| COMPARISON OFFSET paging, page 200 000 of clients | 90171 | 0 | 0.06 | 329.99 |
| COMPARISON keyset paging, the same page | 83 | 0 | 0.01 | 0.11 |
