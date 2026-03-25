FROM node:22-alpine AS builder
WORKDIR /app

# .npmrc intégré pour legacy-peer-deps (Svelte 5 + adapter-node)
COPY .npmrc* ./

COPY package*.json ./
RUN npm install

COPY . .
RUN npm run build

FROM node:22-alpine AS runner
WORKDIR /app

COPY --from=builder /app/build ./build
COPY --from=builder /app/package.json ./
COPY --from=builder /app/node_modules ./node_modules
COPY --from=builder /app/server.js ./server.js

ENV HOST=0.0.0.0
ENV PORT=3000
EXPOSE 3000

CMD ["node", "server.js"]
