FROM node:alpine3.22 AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.80-alpine AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo build --release

FROM alpine:3.22
WORKDIR /app
COPY --from=backend-builder /app/target/release/constellation .
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

ENTRYPOINT [ "./constellation" ]