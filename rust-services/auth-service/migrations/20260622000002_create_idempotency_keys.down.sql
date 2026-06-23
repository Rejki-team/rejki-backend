-- W3D-14 (D4): Rollback idempotency_keys table

DROP INDEX IF EXISTS auth.idx_idempotency_keys_created_at;
DROP TABLE IF EXISTS auth.idempotency_keys;
