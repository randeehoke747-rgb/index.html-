# 1. Create a lib.rs file to register the tonic build output module correctly
cat << 'EOF' > src/lib.rs
pub mod titan_proto {
    tonic::include_proto!("titan.agents.v1");
}
EOF

