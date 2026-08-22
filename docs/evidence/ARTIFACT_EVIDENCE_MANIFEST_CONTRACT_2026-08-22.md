# Artifact evidence manifest contract

状態: `EXTRACTION_CONTRACT_DEFINED / PRODUCT_EVIDENCE_PENDING`

U-01および全要求の証拠は、source commit、X binary、Windows published payload、installer、installed executable、
fresh image、raw log、DB/fixture、独立監査を同じ `artifact_sha256` へ連結する。manifestにはrole、path（秘密を含めない）、
sha256、生成時刻、PID/command、fixture_id、reset_at、timezone、locale、metric、capture size、reviewerを必須とする。
SHA不一致、古い画像の再利用、出所不明の添付、実装者だけのPASSは製品受入を閉じない。
