-- 0001_init · proyectos, membresías y snapshots
-- Destino del adaptador Prisma. No aplicada: requiere PostgreSQL real.
BEGIN;

CREATE TYPE "Role" AS ENUM ('owner', 'member');

CREATE TABLE "projects" (
  "id"         TEXT PRIMARY KEY,
  "name"       TEXT NOT NULL,
  "owner_id"   TEXT NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL
);

CREATE INDEX "projects_owner_id_idx" ON "projects" ("owner_id");

CREATE TABLE "memberships" (
  "project_id" TEXT NOT NULL REFERENCES "projects" ("id") ON DELETE CASCADE,
  "user_id"    TEXT NOT NULL,
  "role"       "Role" NOT NULL,
  PRIMARY KEY ("project_id", "user_id")
);

CREATE INDEX "memberships_user_id_idx" ON "memberships" ("user_id");

CREATE TABLE "snapshots" (
  "id"         TEXT PRIMARY KEY,
  "project_id" TEXT NOT NULL REFERENCES "projects" ("id") ON DELETE CASCADE,
  "run_id"     TEXT NOT NULL,
  "payload"    JSONB NOT NULL,
  "created_at" TIMESTAMPTZ NOT NULL
);

CREATE INDEX "snapshots_project_created_idx" ON "snapshots" ("project_id", "created_at" DESC);

-- Una membresía ausente nunca es acceso: la consulta filtra por project_id y
-- user_id, y el caso "sin fila" se traduce a NOT_FOUND en el puerto.
COMMIT;