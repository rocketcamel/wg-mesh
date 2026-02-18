dev bin="registry":
  bacon dev -- --bin {{bin}}

client target="debug":
  cargo build -p client
  sudo setcap cap_net_admin+ep ./target/{{target}}/client
  ./target/{{target}}/client
