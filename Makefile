.PHONY: setup start stop reset logs status build test clean

COMPOSE := docker compose

setup:
	bash scripts/setup-docker-testnet.sh

build:
	cargo build --release -p aztibase-node

start:
	$(COMPOSE) up --build -d

stop:
	$(COMPOSE) down

reset: stop
	bash scripts/reset-testnet.sh

logs:
	$(COMPOSE) logs -f

status:
	$(COMPOSE) ps
	@echo ""
	@echo "Health checks:"
	@curl -sf http://localhost:9944/health 2>/dev/null && echo "  Validator 1: UP" || echo "  Validator 1: DOWN"
	@curl -sf http://localhost:9945/health 2>/dev/null && echo "  Validator 2: UP" || echo "  Validator 2: DOWN"
	@curl -sf http://localhost:9946/health 2>/dev/null && echo "  Validator 3: UP" || echo "  Validator 3: DOWN"

test:
	cargo test --workspace

clean:
	cargo clean
	rm -rf data/node*/db data/node*/execution_db
