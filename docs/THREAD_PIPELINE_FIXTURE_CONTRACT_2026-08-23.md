# Thread pipeline and paged-content fixture contract（2026-08-23）

状態: `FIXTURE_CONTRACT_DEFINED / PRODUCT_EVIDENCE_PENDING`

対象: `WIN-K-009`, `WIN-M-007`, `WIN-M-010`。

## 1. canonical byte規則

本書のfixture hashは、JSON object keyをUTF-8 byte昇順、余分な空白なし、arrayは記載順、末尾LF 1 byteで
canonical化したbytesのSHA-256である。hashはfixture inputの固定性だけを示し、製品artifactや作業中の
要求文書SHAではない。製品PASSには後段release manifestのartifact SHAとraw outputを別途結合する。

## 2. Stage 1: native DB/rollout収集

valid rowは `r1`（root）と`c1`（child）、valid edgeは`r1→c1`である。両rowはcanonical sessions root内の
non-symlink regular rolloutを持ち、同一cycleの`path -> stable eligible ProcessIdentity set` mapに含まれ、
最後のtask eventは`task_started`である。pathだけの和集合、別cycleのprocess/FD情報、Codex Infoが
観測用にspawnしたapp-server childのFDはliveness証拠にしない。次のcaseをそれぞれvalid baselineから
独立に実行する。

| case | exact rows/edge/event | expected |
| --- | --- | --- |
| `NATIVE_VALID` | rows=`[r1,c1]`; edges=`[[r1,c1]]`; query=`SQLITE_OK`; read_complete=true | accept exactly `{r1,c1}` |
| `NATIVE_DANGLING` | rows=`[r1]`; edges=`[[r1,missing]]` | cycle全体reject、published IDs 0 |
| `NATIVE_CYCLE` | rows=`[r1,c1]`; edges=`[[r1,c1],[c1,r1]]` | cycle全体reject、published IDs 0 |
| `NATIVE_SCHEMA_INVALID` | child ID raw value=`""`（0 scalar） | cycle全体reject、published IDs 0 |
| `NATIVE_QUERY_ERROR` | query result=`SQLITE_CORRUPT` before completion | cycle全体reject、published IDs 0 |
| `NATIVE_PARTIAL` | rows prefix=`[r1]`; read_complete=false | cycle全体reject、published IDs 0 |

上表のcanonical case manifest bytesは次である。

```json
{"cases":[{"edges":[["r1","c1"]],"id":"NATIVE_VALID","query":"SQLITE_OK","read_complete":true,"result":"accept","rows":["r1","c1"]},{"edges":[["r1","missing"]],"id":"NATIVE_DANGLING","result":"reject","rows":["r1"]},{"edges":[["r1","c1"],["c1","r1"]],"id":"NATIVE_CYCLE","result":"reject","rows":["r1","c1"]},{"id":"NATIVE_SCHEMA_INVALID","raw_child_id":"","result":"reject"},{"id":"NATIVE_QUERY_ERROR","query":"SQLITE_CORRUPT","result":"reject"},{"id":"NATIVE_PARTIAL","read_complete":false,"result":"reject","rows_prefix":["r1"]}]}
```

SHA-256は`d9bf9d1429ef91d063c846b5bc77bd92f5fce6992a74ba7e4111cc75af8c7c15`である。
reject caseで`r1`だけをRESTへ渡す、別rootへ接続する、前cycleとmergeすることを禁止する。

### 2.1 process owner・複数collector fixture

`ProcessIdentity`はexact key `pid,starttime_ticks,exe_device,exe_inode`を持つ。FD列挙の前後で同じidentityを
取得できたCodex workloadだけをeligible ownerとする。fixtureのmonitor `artifact_sha=aa...aa`は
Codex Info artifact identityの固定入力であり、製品SHAではない。これを祖先に持つobserver app-serverは、
別Codex Info processがspawnしたものを含めて除外する。publisher admissionは
`profile_account,lease,collector_epoch,cycle_seq`の完全tupleで判定する。

| case | exact境界 | expected |
| --- | --- | --- |
| `OWNER_VALID_TWO_WORKLOADS` | stable workload 2 processが`s1`,`s2`を各open | `s1,s2`を各1回accept |
| `OWNER_OBSERVER_ONLY_TWO_MONITORS` | Codex Info 2 processとobserver child 2 processだけが`stale1`,`stale2`をopen | 正常empty、observer由来thread 0 |
| `OWNER_MIXED_WORKLOAD_OBSERVER` | workloadが`s1`、observerが`stale1`をopen | `s1`だけaccept |
| `OWNER_PID_REUSED` | 同じpidのstarttimeがbefore/afterで変化 | cycle全体reject、last-good保持 |
| `OWNER_ANCESTRY_UNKNOWN` | observer候補の親chainを安全に読めない | cycle全体reject、正常emptyへ変換0 |
| `OWNER_FD_PARTIAL` | eligible候補のFD scanが途中失敗 | cycle全体reject、部分path公開0 |
| `PUBLISHER_ADMISSION` | current/stale leaseとcurrent/stale cycleを順に投入 | current `(L1,7,10)`と`(L1,7,11)`だけaccept、他はno-op |

canonical manifest bytesは次の1行＋末尾LFである。

```json
{"cases":[{"id":"OWNER_VALID_TWO_WORKLOADS","path_owners":{"s1":[{"exe_device":8,"exe_inode":1000,"pid":100,"starttime_ticks":10000}],"s2":[{"exe_device":8,"exe_inode":1000,"pid":101,"starttime_ticks":10001}]},"result":"accept:s1,s2"},{"id":"OWNER_OBSERVER_ONLY_TWO_MONITORS","monitors":[{"artifact_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","exe_device":8,"exe_inode":9000,"pid":200,"starttime_ticks":20000},{"artifact_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","exe_device":8,"exe_inode":9000,"pid":300,"starttime_ticks":30000}],"observers":[{"ancestor":{"artifact_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","exe_device":8,"exe_inode":9000,"pid":200,"starttime_ticks":20000},"identity":{"exe_device":8,"exe_inode":1000,"pid":201,"starttime_ticks":20001},"path":"stale1"},{"ancestor":{"artifact_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","exe_device":8,"exe_inode":9000,"pid":300,"starttime_ticks":30000},"identity":{"exe_device":8,"exe_inode":1000,"pid":301,"starttime_ticks":30001},"path":"stale2"}],"result":"accept-empty"},{"id":"OWNER_MIXED_WORKLOAD_OBSERVER","observer":{"ancestor":{"artifact_sha":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","exe_device":8,"exe_inode":9000,"pid":200,"starttime_ticks":20000},"identity":{"exe_device":8,"exe_inode":1000,"pid":201,"starttime_ticks":20001},"path":"stale1"},"result":"accept:s1","workload":{"identity":{"exe_device":8,"exe_inode":1000,"pid":100,"starttime_ticks":10000},"path":"s1"}},{"after":{"exe_device":8,"exe_inode":1000,"pid":100,"starttime_ticks":10002},"before":{"exe_device":8,"exe_inode":1000,"pid":100,"starttime_ticks":10000},"id":"OWNER_PID_REUSED","result":"reject-cycle"},{"id":"OWNER_ANCESTRY_UNKNOWN","observer_candidate":{"identity":{"exe_device":8,"exe_inode":1000,"pid":201,"starttime_ticks":20001},"parent_read":"denied"},"result":"reject-cycle"},{"fd_scan_complete":false,"id":"OWNER_FD_PARTIAL","identity":{"exe_device":8,"exe_inode":1000,"pid":100,"starttime_ticks":10000},"result":"reject-cycle"},{"events":[{"admission":"current","collector_epoch":7,"cycle_seq":10,"lease":"L1","publisher":"A","result":"accept"},{"admission":"stale-lease","collector_epoch":8,"cycle_seq":11,"lease":"L0","publisher":"B","result":"no-op"},{"admission":"stale-cycle","collector_epoch":7,"cycle_seq":9,"lease":"L1","publisher":"A","result":"no-op"},{"admission":"current","collector_epoch":7,"cycle_seq":11,"lease":"L1","publisher":"A","result":"accept"}],"id":"PUBLISHER_ADMISSION","profile_account":"P1/A1"}]}
```

SHA-256は`860d7ec45d6e53357b6f94201154d5a642fee9611bdb7bb410df5f712ea5f249`である。
全reject/no-op caseは直前last-good ID/order/hash、DB、REST、UIを変更しない。

### 2.2 live/local record failure fixture

live rolloutとlocal usage JSONLは別policyで判定する。liveのtask-stateに影響し得るrecordを隔離して
古い`task_started`をrunningへ流用しない。oversize live recordはbounded streaming parserが
UTF-8/JSON、duplicate/unknown envelope key 0、event kind、liveness非変更tool payloadを完全検証した場合だけ
payloadを隔離できる。invalid UTF-8/JSON、known state-event型不正、証明不能oversizeはcycle全拒否である。

local usageの不正record隔離は、同じfile・reset periodの後続validated cumulative snapshotが
remainingとSOL/TERRA/LUNA各6列を全て持ち、欠落値を覆うcaseだけ許可する。列順はmanifestの
`local_model_column_order`で固定し、後続snapshotのmodel/列欠落はcandidate全rollbackとする。

| case | expected |
| --- | --- |
| `LIVE_OVERSIZE_PROVEN_TOOL_THEN_TERMINAL` | oversize payloadだけ隔離し、後続terminalを受理してactive 0 |
| `LIVE_INVALID_UTF8` / `LIVE_INVALID_JSON` / `LIVE_KNOWN_STATE_TYPE_INVALID` | cycle reject、last-good＋未確認、古いrunning公開0 |
| `LIVE_OVERSIZE_UNPROVEN` | cycle reject、last-good＋未確認 |
| `LIVE_EOF_TAIL` | 次cycleまでhold、last-good＋未確認 |
| `LIVE_VALID_RUNNING` | active 1 |
| `LOCAL_INVALID_COVERED_BY_LATER_CUMULATIVE` | invalid recordだけ隔離し、complete U2をcommit |
| `LOCAL_INVALID_WITHOUT_COVER` | local candidate全rollback |
| `LOCAL_PARTIAL_LATER_SNAPSHOT` | U2_PARTIALをcoverと扱わずlocal candidate全rollback |

canonical manifest bytesは次の1行＋末尾LFである。

```json
{"cases":[{"events":["task_started","tool_payload_oversize_stream_valid_nonliveness","task_completed"],"id":"LIVE_OVERSIZE_PROVEN_TOOL_THEN_TERMINAL","result":"accept-terminal-not-active"},{"events":["task_started","invalid_utf8","task_completed"],"id":"LIVE_INVALID_UTF8","result":"reject-cycle-retain-last-good-unconfirmed"},{"events":["task_started","invalid_json","task_completed"],"id":"LIVE_INVALID_JSON","result":"reject-cycle-retain-last-good-unconfirmed"},{"events":["task_started","task_completed_wrong_type"],"id":"LIVE_KNOWN_STATE_TYPE_INVALID","result":"reject-cycle-retain-last-good-unconfirmed"},{"events":["task_started","oversize_envelope_unproven"],"id":"LIVE_OVERSIZE_UNPROVEN","result":"reject-cycle-retain-last-good-unconfirmed"},{"events":["task_started","eof_unterminated_tail"],"id":"LIVE_EOF_TAIL","result":"hold-next-cycle-retain-last-good-unconfirmed"},{"events":["task_started","tool_payload_valid"],"id":"LIVE_VALID_RUNNING","result":"accept-active"},{"events":[{"id":"U1","model_columns":{"LUNA":[50,5,10,1,1,1],"SOL":[100,10,20,1,1,1],"TERRA":[0,0,0,0,0,0]},"remaining":90,"reset_at":10},"invalid_record",{"id":"U2","model_columns":{"LUNA":[80,8,16,2,2,2],"SOL":[300,30,60,3,3,3],"TERRA":[10,1,2,1,1,1]},"remaining":80,"reset_at":10}],"id":"LOCAL_INVALID_COVERED_BY_LATER_CUMULATIVE","result":"isolate-record-commit-U2"},{"events":[{"id":"U1","model_columns":{"LUNA":[50,5,10,1,1,1],"SOL":[100,10,20,1,1,1],"TERRA":[0,0,0,0,0,0]},"remaining":90,"reset_at":10},"invalid_record"],"id":"LOCAL_INVALID_WITHOUT_COVER","result":"rollback-local-candidate"},{"events":[{"id":"U1","model_columns":{"LUNA":[50,5,10,1,1,1],"SOL":[100,10,20,1,1,1],"TERRA":[0,0,0,0,0,0]},"remaining":90,"reset_at":10},"oversize_unclassified",{"id":"U2_PARTIAL","model_columns":{"SOL":[200,20,40,2,2,2],"TERRA":[10,1,2,1,1,1]},"remaining":80,"reset_at":10}],"id":"LOCAL_PARTIAL_LATER_SNAPSHOT","result":"rollback-local-candidate"}],"local_model_column_order":["input_tokens","cached_input_tokens","output_tokens","input_dollars","cached_input_dollars","output_dollars"]}
```

SHA-256は`76ce097b4412b61b95131c80ae36ddc1768a4fa119591e067f6e3de9d4519d8b`である。
reject/hold caseはthread/localのlast-good resource hashを保持し、quota、DB、REST pair、UI rootの
無関係resourceを変更しない。

## 3. Stage 2: 完全受理REST threads集合

完全なfixture集合は次の3 rowをこのarray順で持つ。各objectはREST v1のexact 12 keyだけを持つ。

```json
[{"context_usage_tokens":100,"context_window_tokens":1000,"created_at":1787356800,"depth":0,"id":"r1","is_subagent":false,"last_user_message_at":1787356860,"model":"gpt-5.6-terra","model_label":"TERRA","parent_thread_id":null,"title":"Root","total_tokens":120},{"context_usage_tokens":50,"context_window_tokens":1000,"created_at":1787356810,"depth":1,"id":"c1","is_subagent":true,"last_user_message_at":1787356870,"model":"gpt-5.6-luna","model_label":"LUNA","parent_thread_id":"r1","title":"Child","total_tokens":60},{"context_usage_tokens":25,"context_window_tokens":1000,"created_at":1787356820,"depth":1,"id":"o1","is_subagent":true,"last_user_message_at":1787356880,"model":"gpt-5.6-sol","model_label":"SOL","parent_thread_id":"z9","title":"Orphan","total_tokens":30}]
```

| bytes | SHA-256（末尾LF込み） |
| --- | --- |
| row `r1` | `80db37afe5b842614ad67e088019d6ab0c8b138fd653595e0780ef254f9a8ce8` |
| row `c1` | `c1477d10c767d7819eacd13cabbfdf137c7595a9c78d358a4e50e72fcf75a389` |
| row `o1` | `b1e1881f9d405c90299254d994071e7a577759de2ee11f6a0577903e28308ce8` |
| complete array | `461e0f28bdde56bc44616358f5d57ad7b706a65f8bb5e0a1b2914fe6a8d2776f` |

`o1.parent_thread_id=z9`だが同じ完全受理集合に`z9`がないため、`o1`だけがvalid orphanである。
wire objectへ`is_orphan`を追加せず、`is_subagent=true`、`depth=1`を保持する。`r1→c1`は接続し、
presentation順は`r1,c1,o1`となる。Stage 1 reject/partial outputからこのorphanを派生させない。

## 4. Stage 3: presentation防御case

各caseは独立に、直前last-good fingerprint=
`461e0f28bdde56bc44616358f5d57ad7b706a65f8bb5e0a1b2914fe6a8d2776f`から開始する。

| case | exact mutation | expected |
| --- | --- | --- |
| `PRESENT_VALID` | IDs=`[r1,c1,o1]`; edges=`[[r1,c1]]`; orphan=`o1` | accept order=`[r1,c1,o1]` |
| `PRESENT_CYCLE` | IDs=`[p1,p2]`; edges=`[[p1,p2],[p2,p1]]` | candidate全体reject |
| `PRESENT_DUPLICATE` | IDs=`[r1,r1]`; second title=`Different` | candidate全体reject |
| `PRESENT_SCHEMA_INVALID` | immutable adapter fault injection: `r1.depth=1025` | candidate全体reject |
| `PRESENT_PARTIAL` | IDs prefix=`[r1]`; complete=false | candidate全体reject |

上表のcanonical case manifest bytesは次である。

```json
{"cases":[{"edges":[["r1","c1"]],"id":"PRESENT_VALID","ids":["r1","c1","o1"],"orphan":"o1","result":"accept"},{"edges":[["p1","p2"],["p2","p1"]],"id":"PRESENT_CYCLE","ids":["p1","p2"],"result":"reject"},{"id":"PRESENT_DUPLICATE","ids":["r1","r1"],"result":"reject","second_title":"Different"},{"id":"PRESENT_SCHEMA_INVALID","mutation":"r1.depth=1025","result":"reject"},{"complete":false,"id":"PRESENT_PARTIAL","ids_prefix":["r1"],"result":"reject"}]}
```

SHA-256は`8b2fe86ea3895547fe3afc77c4c57bb45eb0a7db04fbdf5371c16754876cf07b`である。
全reject caseはvisible IDs/order/hashをlast-goodの`[r1,c1,o1]`/
`461e0f28bdde56bc44616358f5d57ad7b706a65f8bb5e0a1b2914fe6a8d2776f`へ保ち、部分fallback 0とする。

## 5. Threads件数fixture

件数`n`のrowは`i=0..n-1`について、ID=`t`＋3桁zero-pad `i`、title=`Thread {i}`、
parent=null、is_subagent=false、depth=0、model=`gpt-5.6-terra`、model_label=`TERRA`、
total_tokens=`i+10`、context_usage_tokens=`i+1`、context_window_tokens=1000、
created_at=`1787356800+i`、last_user_message_at=`1787356900+i`とする。REST array順をそのままcanonical rankとする。

| n | canonical array SHA-256 |
| ---: | --- |
| 0 | `37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570` |
| 1 | `fd9513efadabaa73395973543ecec5e1b13be87aff5c14ba3ea7657ec04a13c6` |
| 2 | `a20cffed7eade584f39b552bd9ae510ee988089d3437bb63f085e49db9e5432f` |
| 3 | `0d07c20b44bc680680eaf6d78212048fcc4c6e8d2c898b07e5b701e1fdad7601` |
| 4 | `d7c048e2fce4186a2b87b351c363d39c9665ffa024aca23d2ab33a9dcbf3a185` |
| 7 | `140450696f3548b3806d812a97771b0f70281c595f2beba18d669024cd538079` |
| 256 | `d64ee485bbfca33722a1eeb02a3c53300919a34d6b2d9a851be074757f8c0eda` |

page sizeは3、page countは`ceil(n/3)`（n=0だけempty page 1）とし、全pageのID multisetは入力ID
multisetと完全一致する。

## 6. Graph periodとLegal/Help fixture

Graph period `i=0..n-1`はID=`p{i}`、label=`Period {i}`、
start_at=`1786924800-i*604800`、end_at=`1787529600-i*604800`、currentはi=0だけtrueとする。

| n | canonical period array SHA-256 |
| ---: | --- |
| 0 | `37517e5f3dc66819f61f5a7bb8ace1921282415f10551d2defa5c3eb0985b570` |
| 1 | `cf4c97606a48b472e7c71404e805d663db160acf1efeb1fa6b980d02ef5eca39` |
| 4 | `e11b42fa53414066b1779c6442b522e2d6436c24925d47faf88e0f8eef01abf8` |
| 5 | `9396842eea8991487b97d95d614ea5dc2277fef4c626ef655098a50dae9a34d9` |
| 9 | `673798ca4965140d8ce24d4ffdd793c0feff59dcc5129ab1b41549bbed7dd123` |

period page sizeは4、全page ID multisetは入力と一致する。Legal fixtureは6章
`GPL/no-warranty,third-party,font,API-schema,dependency-runtime,distribution`とparagraph IDs
`l00..l05`を1件ずつ持ち、manifest SHA-256は
`7881621f51627aa5b05207b5e7c4e3c3bfcbf93ea08f0f24e0347a0dcc0a1c03`である。
Help fixtureは9章`server-api-silent,recorder-daemon,WSL,remote-SSH,API-check,authentication,
settings-recovery,update-uninstall,diagnostics`とparagraph IDs `h00..h08`を1件ずつ持ち、manifest SHA-256は
`e297d36586988c7b1465002c4f9d1adc17ca25fa61bc2f0e92bc20bb467bd0a6`である。

```json
[{"chapter":"GPL/no-warranty","paragraph_ids":["l00"]},{"chapter":"third-party","paragraph_ids":["l01"]},{"chapter":"font","paragraph_ids":["l02"]},{"chapter":"API-schema","paragraph_ids":["l03"]},{"chapter":"dependency-runtime","paragraph_ids":["l04"]},{"chapter":"distribution","paragraph_ids":["l05"]}]
```

```json
[{"chapter":"server-api-silent","paragraph_ids":["h00"]},{"chapter":"recorder-daemon","paragraph_ids":["h01"]},{"chapter":"WSL","paragraph_ids":["h02"]},{"chapter":"remote-SSH","paragraph_ids":["h03"]},{"chapter":"API-check","paragraph_ids":["h04"]},{"chapter":"authentication","paragraph_ids":["h05"]},{"chapter":"settings-recovery","paragraph_ids":["h06"]},{"chapter":"update-uninstall","paragraph_ids":["h07"]},{"chapter":"diagnostics","paragraph_ids":["h08"]}]
```

fixture paragraph IDはpaging完全性試験用であり、顧客向け本文の代替ではない。製品受入ではreleaseに
同梱する実paragraph bytes/hashを同じmissing/extra/duplicate式へ入力する。

## 7. 三値式

- 抽出PASS: 本書path、case ID、canonicalization、入力、期待、hash、行契約joinがすべて定義済み。
- 製品PASS: 同一release artifact SHAのraw各stage/output/pageが全positive/negative期待と一致。
- FAIL: 入力hash不一致、期待との矛盾、部分公開、missing/extra/duplicate、wire field追加。
- INCONCLUSIVE: raw stage、artifact SHA、capture時刻、独立reviewerのいずれかが欠落・stale・別release。
