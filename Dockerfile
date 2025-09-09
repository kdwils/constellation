FROM node:22.19.0 AS frontend-builder
WORKDIR /app/frontend
COPY frontend/package.json frontend/package-lock.json* ./
RUN npm ci
COPY frontend/ ./
RUN npm run build

FROM rust:1.89.0 AS backend-builder
WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY . .
RUN cargo build --release

FROM gcr.io/distroless/static-debian12:nonroot
WORKDIR /app
COPY --from=backend-builder /app/target/release/constellation .
COPY --from=frontend-builder /app/frontend/dist ./frontend/dist

ENTRYPOINT [ "./constellation" ]