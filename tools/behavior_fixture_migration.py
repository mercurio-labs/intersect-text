"""Model-owned offline fixture source conversion; no Text host dependency."""
import copy
import hashlib
import json


def quote(value):
    return json.dumps(value, ensure_ascii=False)


def name(value):
    return "`" + value.replace("`", "``") + "`"


def constant(value):
    if value is None:
        return "none"
    if isinstance(value, dict):
        return "{" + ", ".join(f"{name(k)}: {constant(v)}" for k, v in value.items()) + "}"
    if isinstance(value, list):
        return "[" + ", ".join(map(constant, value)) + "]"
    return quote(value)


def render(rules, namespace):
    def fields(value, allowed):
        unknown = set(value) - set(allowed)
        if unknown:
            raise ValueError(f'Unsupported rule fields: {sorted(unknown)}')
    fields(rules, ['version', 'scope', 'derived', 'validations', 'initializers', 'rule_conversions'])
    if rules["version"] != 1 or rules["scope"] != "unique_names":
        raise ValueError("Unsupported fixture rule version/scope")
    lines = ['rcore 1;', 'package fixtureProviders version "1";']
    if any(rules.get(key) for key in ["derived", "validations", "initializers"]):
        if not namespace:
            raise ValueError("Model provider fixture needs a namespace")
        lines.append(f'import model {quote(namespace)} as m;')
    for rule in rules["derived"]:
        fields(rule, ['class', 'feature', 'expression'])
        target = "m." + name(rule["class"]) + "." + name(rule["feature"])
        expression = rule["expression"]
        fields(expression, {'stored': ['op', 'feature', 'writable'], 'count': ['op', 'feature'],
                            'no_incoming': ['op', 'class', 'feature']}.get(expression['op'], []))
        feature = name(expression["feature"])
        if expression["op"] == "stored":
            lines.append(f'projection {target} from {feature}' + (' writable' if expression.get('writable') else '') + ';')
        elif expression["op"] == "count":
            lines.append(f'derive {target} using count({feature});')
        elif expression["op"] == "no_incoming":
            lines.append(f'derive {target} using noIncoming(m.{name(expression["class"])}.{feature});')
        else:
            raise ValueError(f"Unknown derivation {expression}")
    for index, rule in enumerate(rules["validations"]):
        fields(rule, ['class', 'feature', 'max_incoming', 'message'])
        owner = "m." + name(rule["class"])
        feature = rule["feature"]
        lines += [f'validate {owner}.rule{index}(self) {{',
                  f'  require size(incoming::<{owner}>(self.{name(feature)}, {quote(feature)})) <= {rule["max_incoming"]}',
                  f'    code "rule{index}" message {quote(rule["message"])};', '}']
    for rule in rules.get("initializers", []):
        fields(rule, ['class', 'provider', 'members', 'values'])
        provider = rule["provider"]
        fields(provider, ['name', 'version'])
        if set(rule.get('values') or {}) - set(rule['members']):
            raise ValueError('Initializer values must match declared members')
        lines.append(f'initialize m.{name(rule["class"])} using {quote(provider["name"])} version {quote(provider["version"])} {{')
        for member in rule["members"]:
            assignment = " = " + constant(rule["values"][member]) if member in (rule.get("values") or {}) else ""
            lines.append(f'  {name(member)}{assignment};')
        lines.append('}')
    for rule in rules.get("rule_conversions", []):
        fields(rule, ['context', 'rule', 'declaration', 'datatype', 'provider', 'keywords'])
        span, provider = rule["declaration"], rule["provider"]
        fields(span, ['source', 'start', 'end'])
        fields(provider, ['name', 'version'])
        source = span["source"]
        fields(source, ['uri', 'sha256'])
        lines += [f'convert {quote(rule["context"])} rule {quote(rule["rule"])}',
                  f'  from {quote(source["uri"])} sha256 {quote(source["sha256"])} span {span["start"]} {span["end"]}',
                  f'  to {quote(rule["datatype"])} using {quote(provider["name"])} version {quote(provider["version"])} {{']
        lines += [f'  {quote(key)} = {quote(value)};' for key, value in rule["keywords"].items()]
        lines.append('}')
    return "\n".join(lines) + "\n"


def canonicalize_bundle_value(value):
    """Convert embedded rule sources and rebind all exact source references.

    The caller must recompile any retained model/schema snapshots afterwards.
    """
    value = copy.deepcopy(value)
    identities = {}

    def visit(node):
        if isinstance(node, list):
            for item in node:
                visit(item)
        elif isinstance(node, dict):
            if isinstance(node.get('bundle'), dict):
                for source in node['bundle'].get('sources', []):
                    if source.get('kind') == 'rcore_behavior' and source['text'].lstrip().startswith('{'):
                        original = copy.deepcopy(source['id'])
                        source['text'] = render(json.loads(source['text']), node.get('namespace'))
                        source['id']['sha256'] = hashlib.sha256(source['text'].encode('utf-8')).hexdigest()
                        key = (original['uri'], original['sha256'])
                        if key in identities and identities[key] != source['id']:
                            raise ValueError('One behavior source cannot select different model namespaces')
                        identities[key] = copy.deepcopy(source['id'])
            for item in node.values():
                visit(item)

    def remap(node):
        if isinstance(node, list):
            return [remap(item) for item in node]
        if isinstance(node, dict):
            if set(node) == {'uri', 'sha256'}:
                return identities.get((node['uri'], node['sha256']), node)
            return {key: remap(item) for key, item in node.items()}
        return node

    visit(value)
    return remap(value)


