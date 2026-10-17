#!/usr/bin/env python3
"""Read-only, nonzero-coverage alternative to embedded bd doctor/lint.

Checks issue records and the complete dependency graph through supported bd
JSON commands. Does not claim low-level Dolt integrity or replace a DB backup.
"""
import json
from pathlib import Path
import subprocess

ROOT = Path(__file__).resolve().parents[1]


def bd(*args):
    return json.loads(subprocess.check_output(['bd', *args, '--json'], cwd=ROOT, text=True))


def audit():
    issues = bd('list', '--all', '--limit', '0')
    assert issues, 'Empty issue list: refusing a zero-coverage pass'
    records = {i['id']: i for i in issues}
    assert len(records) == len(issues), 'Duplicate IDs'
    edges = {key: [] for key in records}
    for key, issue in records.items():
        for field in ('title', 'description', 'acceptance_criteria'):
            assert issue.get(field, '').strip(), f'{key}: missing {field}'
        assert issue['status'] in ('open', 'in_progress', 'blocked', 'deferred', 'closed', 'pinned', 'hooked'), key
        if issue['status'] == 'closed':
            assert issue.get('close_reason', '').strip(), f'{key}: no closure evidence'
        # Independent readback catches listing/hydration omissions.
        shown = bd('show', key)[0]
        for field in ('id', 'title', 'description', 'acceptance_criteria', 'status'):
            assert shown[field] == issue[field], f'{key}: inconsistent {field}'
        for dependency in issue.get('dependencies', []):
            target = dependency['depends_on_id']
            assert target in records, f'{key}: dangling dependency {target}'
            if dependency['type'] in ('blocks', 'parent-child'):
                edges[key].append(target)
    visiting, visited = set(), set()

    def visit(key):
        assert key not in visiting, f'Dependency cycle at {key}'
        if key in visited:
            return
        visiting.add(key)
        for target in edges[key]:
            visit(target)
        visiting.remove(key)
        visited.add(key)

    for key in records:
        visit(key)
    assert not bd('dep', 'cycles'), 'bd reports dependency cycles'
    result = dict(issues_checked=len(issues), edges_checked=sum(map(len, edges.values())),
                  open_issues=[i['id'] for i in issues if i['status'] != 'closed'],
                  scope='Issue fields, independent readback, reference integrity and graph cycles; not physical Dolt storage integrity')
    output = ROOT / '.artifacts/beads-audit.json'
    output.parent.mkdir(exist_ok=True)
    output.write_text(json.dumps(result, indent=2) + '\n')
    print(json.dumps(result, indent=2))


if __name__ == '__main__':
    audit()
