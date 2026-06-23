-- ─────────────────────────────────────────────────────────────────────────────
-- Rejki Backend — Init Databases untuk Shared PostgreSQL
-- 
-- Container PostgreSQL ini dipakai BERSAMA oleh dev & prod environment.
-- Masing-masing environment punya database sendiri agar tidak tercampur.
-- Dieksekusi otomatis via docker-entrypoint-initdb.d
-- ─────────────────────────────────────────────────────────────────────────────

-- 1. Buat database untuk masing-masing environment
CREATE DATABASE rejki_dev  WITH ENCODING 'UTF8' LC_COLLATE 'en_US.utf8' LC_CTYPE 'en_US.utf8';
CREATE DATABASE rejki_prod WITH ENCODING 'UTF8' LC_COLLATE 'en_US.utf8' LC_CTYPE 'en_US.utf8';

-- 2. Schema untuk DEV database
\c rejki_dev
CREATE SCHEMA IF NOT EXISTS auth;
CREATE SCHEMA IF NOT EXISTS user_svc;
CREATE SCHEMA IF NOT EXISTS chat;
CREATE SCHEMA IF NOT EXISTS notification;
CREATE SCHEMA IF NOT EXISTS iklan_pekerjaan;
CREATE SCHEMA IF NOT EXISTS iklan_pekerja;
CREATE SCHEMA IF NOT EXISTS iklan_barang_bekas;
CREATE SCHEMA IF NOT EXISTS iklan_pelatihan;
CREATE SCHEMA IF NOT EXISTS comms;
CREATE SCHEMA IF NOT EXISTS report;
CREATE SCHEMA IF NOT EXISTS storage;
CREATE SCHEMA IF NOT EXISTS region;
CREATE SCHEMA IF NOT EXISTS analytics;
CREATE SCHEMA IF NOT EXISTS corporate_comms;

GRANT USAGE, CREATE ON SCHEMA auth               TO rejki;
GRANT USAGE, CREATE ON SCHEMA user_svc           TO rejki;
GRANT USAGE, CREATE ON SCHEMA chat               TO rejki;
GRANT USAGE, CREATE ON SCHEMA notification       TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pekerjaan    TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pekerja      TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_barang_bekas TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pelatihan    TO rejki;
GRANT USAGE, CREATE ON SCHEMA comms              TO rejki;
GRANT USAGE, CREATE ON SCHEMA report             TO rejki;
GRANT USAGE, CREATE ON SCHEMA storage            TO rejki;
GRANT USAGE, CREATE ON SCHEMA region             TO rejki;
GRANT USAGE, CREATE ON SCHEMA analytics          TO rejki;
GRANT USAGE, CREATE ON SCHEMA corporate_comms    TO rejki;

-- 3. Schema untuk PROD database (identik dengan dev)
\c rejki_prod
CREATE SCHEMA IF NOT EXISTS auth;
CREATE SCHEMA IF NOT EXISTS user_svc;
CREATE SCHEMA IF NOT EXISTS chat;
CREATE SCHEMA IF NOT EXISTS notification;
CREATE SCHEMA IF NOT EXISTS iklan_pekerjaan;
CREATE SCHEMA IF NOT EXISTS iklan_pekerja;
CREATE SCHEMA IF NOT EXISTS iklan_barang_bekas;
CREATE SCHEMA IF NOT EXISTS iklan_pelatihan;
CREATE SCHEMA IF NOT EXISTS comms;
CREATE SCHEMA IF NOT EXISTS report;
CREATE SCHEMA IF NOT EXISTS storage;
CREATE SCHEMA IF NOT EXISTS region;
CREATE SCHEMA IF NOT EXISTS analytics;
CREATE SCHEMA IF NOT EXISTS corporate_comms;

GRANT USAGE, CREATE ON SCHEMA auth               TO rejki;
GRANT USAGE, CREATE ON SCHEMA user_svc           TO rejki;
GRANT USAGE, CREATE ON SCHEMA chat               TO rejki;
GRANT USAGE, CREATE ON SCHEMA notification       TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pekerjaan    TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pekerja      TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_barang_bekas TO rejki;
GRANT USAGE, CREATE ON SCHEMA iklan_pelatihan    TO rejki;
GRANT USAGE, CREATE ON SCHEMA comms              TO rejki;
GRANT USAGE, CREATE ON SCHEMA report             TO rejki;
GRANT USAGE, CREATE ON SCHEMA storage            TO rejki;
GRANT USAGE, CREATE ON SCHEMA region             TO rejki;
GRANT USAGE, CREATE ON SCHEMA analytics          TO rejki;
GRANT USAGE, CREATE ON SCHEMA corporate_comms    TO rejki;
