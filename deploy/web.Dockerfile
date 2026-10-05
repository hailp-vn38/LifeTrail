# LifeTrail web image: built Vue SPA served by Nginx.
#
# Nginx serves `/` and `/assets/*` from the built SPA and reverse-proxies
# `/api/*` to the `server` service on the internal Compose network, so the
# browser keeps a single same-origin entrypoint at http://<LAN-IP>:8080.
# The production runtime contains Nginx + static assets only (no Node.js).
FROM node:22-bookworm-slim AS builder

WORKDIR /app

COPY web/package.json web/package-lock.json ./
RUN npm ci

COPY web/ ./

ARG VITE_MAP_STYLE_URL
ARG VITE_MAPTILER_KEY

ENV VITE_MAP_STYLE_URL=$VITE_MAP_STYLE_URL
ENV VITE_MAPTILER_KEY=$VITE_MAPTILER_KEY

RUN npm run build

FROM nginx:alpine

COPY deploy/nginx.conf /etc/nginx/conf.d/default.conf
COPY --from=builder /app/dist /usr/share/nginx/html

EXPOSE 80
