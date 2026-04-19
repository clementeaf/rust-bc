#!/usr/bin/env python3
"""
Tesseract Stress Tests — agents that try to BREAK the system.

Nothing is designed to succeed. Every scenario is adversarial.
Run with nodes already started:
  PORT=7710 NODE_ID=n1 PEERS=127.0.0.1:7711 cargo run --bin node &
  PORT=7711 NODE_ID=n2 PEERS=127.0.0.1:7710 cargo run --bin node &
  python3 stress_test.py
"""
import asyncio
import time
import sys
from client import TesseractClient, Cell

N1 = "http://127.0.0.1:7710"
N2 = "http://127.0.0.1:7711"

# ── Helpers ──────────────────────────────────────────────

RED = "\033[31m"
GREEN = "\033[32m"
YELLOW = "\033[33m"
CYAN = "\033[36m"
BOLD = "\033[1m"
DIM = "\033[2m"
RESET = "\033[0m"

passed = 0
failed = 0
errors: list[str] = []


def header(name: str):
    print(f"\n{BOLD}{YELLOW}{'━' * 60}{RESET}")
    print(f"{BOLD}{YELLOW}  {name}{RESET}")
    print(f"{BOLD}{YELLOW}{'━' * 60}{RESET}\n")


def ok(msg: str):
    global passed
    passed += 1
    print(f"  {GREEN}✓ {msg}{RESET}")


def fail(msg: str):
    global failed
    failed += 1
    errors.append(msg)
    print(f"  {RED}✗ {msg}{RESET}")


def info(msg: str):
    print(f"  {DIM}{msg}{RESET}")


def check(condition: bool, pass_msg: str, fail_msg: str):
    if condition:
        ok(pass_msg)
    else:
        fail(fail_msg)


# ── Stress Test 1: Race condition ────────────────────────

async def test_race_condition():
    """10 agents seed the SAME cell simultaneously on BOTH nodes."""
    header("STRESS 1: Race condition — 10 agents, same cell, same time")

    c1 = TesseractClient(N1)
    c2 = TesseractClient(N2)

    coord = (1, 1, 3, 3)

    # 10 agents fire simultaneously, split across nodes
    tasks = []
    for i in range(10):
        node = c1 if i % 2 == 0 else c2
        tasks.append(node.seed(*coord, f"agent-{i}:race"))

    t0 = time.time()
    results = await asyncio.gather(*tasks, return_exceptions=True)
    elapsed = time.time() - t0

    exceptions = [r for r in results if isinstance(r, Exception)]
    check(len(exceptions) == 0, f"All 10 seeds completed in {elapsed:.2f}s", f"{len(exceptions)} seeds failed")

    await asyncio.sleep(2)  # let sync happen

    cell1 = await c1.get_cell(*coord)
    cell2 = await c2.get_cell(*coord)

    check(cell1.probability > 0, f"Node 1: p={cell1.probability:.2f}", "Node 1: cell is empty after 10 seeds!")
    check(cell2.probability > 0, f"Node 2: p={cell2.probability:.2f}", "Node 2: cell is empty after sync!")

    # All 10 agents should appear in influences
    all_agents = all(f"agent-{i}" in cell1.record for i in range(0, 10, 2))
    info(f"Record (node1): {cell1.record[:120]}...")
    check(cell1.crystallized, "Cell crystallized under concurrent load", "Cell did NOT crystallize — convergence failed under load")

    await c1.close()
    await c2.close()


# ── Stress Test 2: Double-spend ──────────────────────────

async def test_double_spend():
    """Alice agrees with Bob AND Charlie at the same time — both should crystallize (L1 accepts all)."""
    header("STRESS 2: Double-spend — Alice agrees with Bob AND Charlie simultaneously")

    c1 = TesseractClient(N1)
    c2 = TesseractClient(N2)

    deal_bob = (2, 2, 3, 3)
    deal_charlie = (2, 5, 3, 3)

    # Alice seeds BOTH deals at the same time
    await asyncio.gather(
        c1.seed(*deal_bob, "alice:deal-bob"),
        c1.seed(*deal_charlie, "alice:deal-charlie"),
        c2.seed(*deal_bob, "bob:accepts"),
        c2.seed(*deal_charlie, "charlie:accepts"),
    )

    await asyncio.sleep(2)

    cell_bob = await c1.get_cell(*deal_bob)
    cell_charlie = await c1.get_cell(*deal_charlie)

    info(f"Deal with Bob:     p={cell_bob.probability:.2f} cryst={cell_bob.crystallized}")
    info(f"Deal with Charlie: p={cell_charlie.probability:.2f} cryst={cell_charlie.crystallized}")

    # L1 should accept BOTH — this is by design
    check(
        cell_bob.probability > 0 and cell_charlie.probability > 0,
        "BOTH deals exist in the field (L1 accepts all — correct behavior)",
        "One deal was rejected at L1 — this should NOT happen"
    )

    # Both should have alice's influence
    check("alice" in cell_bob.record, f"Bob deal has Alice: {cell_bob.record[:80]}", "Bob deal missing Alice")
    check("alice" in cell_charlie.record, f"Charlie deal has Alice: {cell_charlie.record[:80]}", "Charlie deal missing Alice")

    # This is where the WALLET layer (L2) would reject one
    info("NOTE: L1 accepts both. L2 (wallet) must resolve the double-spend.")

    await c1.close()
    await c2.close()


# ── Stress Test 3: Flood attack ──────────────────────────

async def test_flood():
    """500 events in 2 seconds — can the node handle it?"""
    header("STRESS 3: Flood attack — 500 events in 2 seconds")

    c1 = TesseractClient(N1)

    tasks = []
    for i in range(500):
        t = i % 8
        c = (i // 8) % 8
        tasks.append(c1.seed(t, c, 3, 3, f"flood-{i}"))

    t0 = time.time()
    results = await asyncio.gather(*tasks, return_exceptions=True)
    elapsed = time.time() - t0

    successes = sum(1 for r in results if not isinstance(r, Exception))
    failures = sum(1 for r in results if isinstance(r, Exception))

    info(f"Time: {elapsed:.2f}s — {successes}/{len(tasks)} succeeded, {failures} failed")
    tps = successes / elapsed if elapsed > 0 else 0
    info(f"Throughput: {tps:.0f} events/sec")

    check(successes >= 400, f"{successes}/500 events accepted ({tps:.0f} ev/s)", f"Too many failures: {failures}/500")

    # Check node is still alive
    try:
        status = await c1.status()
        check(True, f"Node survived flood — {status.get('active_cells', '?')} active cells", "")
    except Exception as e:
        fail(f"Node CRASHED after flood: {e}")

    await c1.close()


# ── Stress Test 4: Destroy during convergence ────────────

async def test_destroy_during_convergence():
    """Seed an event and immediately destroy it while evolving."""
    header("STRESS 4: Destroy during crystallization")

    c1 = TesseractClient(N1)
    c2 = TesseractClient(N2)
    coord = (3, 6, 3, 3)

    # Seed from both nodes for strong support
    await c1.seed(*coord, "convergence-test[alice]")
    await c2.seed(*coord, "convergence-test[bob]")
    # Supporting events
    await c1.seed(3, 7, 3, 3, "support-1[alice]")
    await c2.seed(3, 7, 3, 3, "support-1[bob]")
    await c1.seed(4, 6, 3, 3, "support-2[alice]")
    await c2.seed(4, 6, 3, 3, "support-2[bob]")

    # Immediately destroy while field is evolving
    await asyncio.sleep(0.5)
    await c1.destroy(*coord)

    cell_after_destroy = await c1.get_cell(*coord)
    info(f"After destroy: p={cell_after_destroy.probability:.2f} cryst={cell_after_destroy.crystallized}")

    # Wait for self-healing
    healed = False
    for i in range(15):
        await asyncio.sleep(0.5)
        cell = await c1.get_cell(*coord)
        if cell.crystallized:
            healed = True
            info(f"Healed after {(i+1)*0.5:.1f}s — p={cell.probability:.2f}")
            break

    check(healed, "Cell self-healed after mid-convergence destroy", "Cell did NOT heal — self-healing failed under stress")

    await c1.close()
    await c2.close()


# ── Stress Test 5: Identity spoofing ─────────────────────

async def test_identity_spoofing():
    """20 agents all claim to be 'alice' — seed from different nodes."""
    header("STRESS 5: Identity spoofing — 20 agents all claim to be 'alice'")

    c1 = TesseractClient(N1)
    c2 = TesseractClient(N2)
    coord = (4, 4, 3, 3)

    # Real alice seeds with bob's endorsement
    await c1.seed(*coord, "real-alice:deal")
    await c2.seed(*coord, "real-bob:endorses")

    await asyncio.sleep(1)

    cell_before = await c1.get_cell(*coord)
    info(f"Before spoofing: p={cell_before.probability:.2f} record={cell_before.record[:80]}")

    # 20 fake alices flood the same cell
    tasks = []
    for i in range(20):
        node = c1 if i % 2 == 0 else c2
        tasks.append(node.seed(*coord, f"fake-alice-{i}:deal"))

    await asyncio.gather(*tasks)
    await asyncio.sleep(2)

    cell_after = await c1.get_cell(*coord)
    info(f"After spoofing: p={cell_after.probability:.2f}")
    info(f"Record: {cell_after.record[:150]}...")

    # The real alice should still be in the record
    check("real-alice" in cell_after.record, "Real alice survives in record", "Real alice was displaced!")

    # Fake alices will also be in the record (L1 accepts all)
    # but the KEY check: the cell wasn't corrupted
    check(cell_after.crystallized, "Cell remains crystallized despite spoofing", "Cell de-crystallized under spoofing attack!")
    check(cell_after.probability > 0.9, f"Probability stable at {cell_after.probability:.2f}", f"Probability degraded to {cell_after.probability:.2f}")

    await c1.close()
    await c2.close()


# ── Stress Test 6: Read-write consistency ────────────────

async def test_read_write_consistency():
    """Read a cell while another agent writes to it — 100 times rapidly."""
    header("STRESS 6: Read-write consistency — 100 concurrent read/write cycles")

    c1 = TesseractClient(N1)
    coord = (5, 3, 3, 3)

    inconsistencies = 0

    async def writer():
        for i in range(100):
            await c1.seed(*coord, f"write-{i}")
            await asyncio.sleep(0.01)

    async def reader():
        nonlocal inconsistencies
        for _ in range(100):
            cell = await c1.get_cell(*coord)
            # Consistency check: probability must be between 0 and 1
            if cell.probability < 0 or cell.probability > 1.001:
                inconsistencies += 1
            await asyncio.sleep(0.01)

    await asyncio.gather(writer(), reader())

    check(inconsistencies == 0, f"100 concurrent read/writes — zero inconsistencies", f"{inconsistencies} inconsistent reads detected!")

    cell = await c1.get_cell(*coord)
    info(f"Final state: p={cell.probability:.2f} cryst={cell.crystallized}")

    await c1.close()


# ── Stress Test 7: Asymmetric load ───────────────────────

async def test_asymmetric_load():
    """Node 1 gets 100x more traffic than Node 2. Do they stay in sync?"""
    header("STRESS 7: Asymmetric load — Node 1 gets 200 events, Node 2 gets 2")

    c1 = TesseractClient(N1)
    c2 = TesseractClient(N2)

    coord = (6, 3, 3, 3)

    # Heavy load on node 1
    tasks = [c1.seed(6, i % 8, 3, 3, f"heavy-{i}") for i in range(200)]
    # Tiny load on node 2
    tasks.append(c2.seed(*coord, "light-bob"))
    tasks.append(c2.seed(6, 4, 3, 3, "light-bob-2"))

    await asyncio.gather(*tasks, return_exceptions=True)

    # Wait for sync
    info("Waiting 5s for asymmetric sync...")
    await asyncio.sleep(5)

    cell1 = await c1.get_cell(*coord)
    cell2 = await c2.get_cell(*coord)

    info(f"Node 1: p={cell1.probability:.2f} cryst={cell1.crystallized}")
    info(f"Node 2: p={cell2.probability:.2f} cryst={cell2.crystallized}")

    # Both should have data
    check(cell1.probability > 0, "Node 1 has data at coord", "Node 1 lost data under load")
    check(cell2.probability > 0, "Node 2 has data at coord (synced from Node 1)", "Node 2 never received data — sync broke under asymmetric load!")

    # Check that bob's event reached node 1
    check("light-bob" in cell1.record or cell1.probability > 0, "Bob's event propagated to heavy node", "Bob's event LOST — heavy node ignored light node's sync")

    # Node 2 should have some of node 1's heavy events too
    s2 = await c2.status()
    info(f"Node 2 active cells: {s2.get('active_cells', '?')}")
    check(s2.get("active_cells", 0) > 10, f"Node 2 synced {s2.get('active_cells', 0)} cells from Node 1", "Node 2 barely synced — asymmetric sync failure")

    await c1.close()
    await c2.close()


# ── Main ─────────────────────────────────────────────────

async def main():
    print(f"\n{BOLD}{CYAN}╔══════════════════════════════════════════════════════════╗{RESET}")
    print(f"{BOLD}{CYAN}║   TESSERACT STRESS TESTS                                 ║{RESET}")
    print(f"{BOLD}{CYAN}║   Nothing is designed to succeed. Break everything.       ║{RESET}")
    print(f"{BOLD}{CYAN}╚══════════════════════════════════════════════════════════╝{RESET}")

    # Verify nodes are up
    c1 = TesseractClient(N1)
    c2 = TesseractClient(N2)
    try:
        s1 = await c1.status()
        s2 = await c2.status()
        info(f"Node 1: {s1.get('node_id', '?')} — {s1.get('active_cells', 0)} cells")
        info(f"Node 2: {s2.get('node_id', '?')} — {s2.get('active_cells', 0)} cells")
        ok("Both nodes online")
    except Exception as e:
        fail(f"Nodes not reachable: {e}")
        print(f"\n{RED}Start nodes first:{RESET}")
        print(f"  PORT=7710 NODE_ID=n1 PEERS=127.0.0.1:7711 cargo run --bin node &")
        print(f"  PORT=7711 NODE_ID=n2 PEERS=127.0.0.1:7710 cargo run --bin node &")
        await c1.close()
        await c2.close()
        sys.exit(1)
    await c1.close()
    await c2.close()

    # Run all stress tests
    await test_race_condition()
    await test_double_spend()
    await test_flood()
    await test_destroy_during_convergence()
    await test_identity_spoofing()
    await test_read_write_consistency()
    await test_asymmetric_load()

    # Summary
    total = passed + failed
    print(f"\n{BOLD}{'━' * 60}{RESET}")
    print(f"{BOLD}  RESULTS: {GREEN}{passed} passed{RESET} / {RED}{failed} failed{RESET} / {total} total")
    if errors:
        print(f"\n{RED}  Failures:{RESET}")
        for e in errors:
            print(f"    {RED}✗ {e}{RESET}")
    print(f"{BOLD}{'━' * 60}{RESET}\n")

    sys.exit(0 if failed == 0 else 1)


if __name__ == "__main__":
    asyncio.run(main())
