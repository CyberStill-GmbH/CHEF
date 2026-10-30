# Empaquetado reproducible de la API Node.
# Genera el cliente Prisma en build y lo copia al runtime, de modo que con
# DATABASE_URL la API persista en PostgreSQL. El motor Go aún no existe: cuando
# exista, se suma una etapa de build y se copia el binario estático.

FROM node:24-bookworm-slim AS build
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci
COPY tsconfig.json ./
COPY packages ./packages
COPY apps/api ./apps/api
COPY apps/cli ./apps/cli
COPY scripts ./scripts
COPY tests ./tests
COPY docs/contracts ./docs/contracts
COPY fixtures ./fixtures
RUN npm run build
# Cliente Prisma generado dentro de la imagen (motor de consulta para esta base).
RUN npx prisma generate --schema apps/api/prisma/schema.prisma

FROM node:24-bookworm-slim AS runtime
ENV NODE_ENV=production
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci --omit=dev && npm cache clean --force
COPY --from=build /app/dist ./dist
COPY --from=build /app/docs/contracts ./docs/contracts
COPY --from=build /app/fixtures/scope ./fixtures/scope
COPY --from=build /app/apps/api/prisma ./apps/api/prisma
# El cliente generado (y su motor) no lo produce `npm ci`: se trae del build.
COPY --from=build /app/node_modules/.prisma ./node_modules/.prisma
COPY --from=build /app/node_modules/@prisma/client ./node_modules/@prisma/client
USER node
EXPOSE 8787
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s --retries=3 \
  CMD node -e "fetch('http://127.0.0.1:' + (process.env.PORT || 8787) + '/health').then((r) => process.exit(r.ok ? 0 : 1)).catch(() => process.exit(1))"
CMD ["node", "dist/apps/api/src/main.js"]