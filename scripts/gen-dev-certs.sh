#!/usr/bin/env bash
set -euo pipefail

OUT_DIR="${1:-certs}"
DAYS=365
CN="sentinel-dev"

mkdir -p "$OUT_DIR"

echo "==> Generating CA key + cert"
openssl req -x509 -newkey rsa:4096 -nodes \
  -keyout "$OUT_DIR/ca-key.pem" \
  -out "$OUT_DIR/ca-cert.pem" \
  -days "$DAYS" \
  -subj "/CN=${CN}-ca"

echo "==> Generating server key + CSR"
openssl req -newkey rsa:4096 -nodes \
  -keyout "$OUT_DIR/server-key.pem" \
  -out "$OUT_DIR/server.csr" \
  -subj "/CN=${CN}"

cat > "$OUT_DIR/ext.cnf" <<EOF
[v3_req]
subjectAltName = @alt_names
[alt_names]
DNS.1 = localhost
IP.1  = 127.0.0.1
IP.2  = ::1
EOF

echo "==> Signing server cert with CA"
openssl x509 -req \
  -in "$OUT_DIR/server.csr" \
  -CA "$OUT_DIR/ca-cert.pem" \
  -CAkey "$OUT_DIR/ca-key.pem" \
  -CAcreateserial \
  -out "$OUT_DIR/server-cert.pem" \
  -days "$DAYS" \
  -extfile "$OUT_DIR/ext.cnf" \
  -extensions v3_req

rm -f "$OUT_DIR/server.csr" "$OUT_DIR/ext.cnf" "$OUT_DIR/ca-cert.srl"

echo "==> Certificates generated in $OUT_DIR/"
echo "    ca-cert.pem      (CA certificate)"
echo "    ca-key.pem       (CA private key)"
echo "    server-cert.pem  (server certificate)"
echo "    server-key.pem   (server private key)"
