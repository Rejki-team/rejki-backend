-- Dieksekusi otomatis saat container PostgreSQL pertama kali dibuat.
-- Volume mount: ./scripts/init-schemas.sql:/docker-entrypoint-initdb.d/init.sql

CREATE SCHEMA IF NOT EXISTS auth;
CREATE SCHEMA IF NOT EXISTS user_svc;
CREATE SCHEMA IF NOT EXISTS chat;
CREATE SCHEMA IF NOT EXISTS notification;
CREATE SCHEMA IF NOT EXISTS iklan_pekerjaan;
CREATE SCHEMA IF NOT EXISTS iklan_pekerja;
CREATE SCHEMA IF NOT EXISTS iklan_barang_bekas;
CREATE SCHEMA IF NOT EXISTS iklan_pelatihan;

GRANT USAGE, CREATE ON SCHEMA auth               TO rejki;
GRANT USAGE, CREATE ON SCHEMA user_svc           TO rejki;
GRANT USAGE, CREATE ON SCHEMA chat               TO rejki;
GRANT USAGE, CREATE ON SCHEMA notification       TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pekerjaan    TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pekerja      TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_barang_bekas TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pelatihan    TO rejki;
