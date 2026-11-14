// Independently authored test harness; Java is never a production dependency.
package intersect.oracle;
import java.util.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.generator.parser.antlr.FirstSetComputer;
public final class FirstSetOracle {
 static final XtextFactory F=XtextFactory.eINSTANCE;
 static Keyword keyword(String value){Keyword k=F.createKeyword();k.setValue(value);return k;}
 static Group group(AbstractElement... values){Group g=F.createGroup();g.getElements().addAll(Arrays.asList(values));return g;}
 static Alternatives choice(AbstractElement... values){Alternatives a=F.createAlternatives();a.getElements().addAll(Arrays.asList(values));return a;}
 static RuleCall call(AbstractRule rule){RuleCall c=F.createRuleCall();c.setRule(rule);return c;}
 static ParserRule rule(String name,AbstractElement body){ParserRule r=F.createParserRule();r.setName(name);r.setAlternatives(body);return r;}
 static AbstractElement optional(AbstractElement value){value.setCardinality("?");return value;}
 static void observe(String name,AbstractElement element){
  List<String> labels=new ArrayList<>();
  for(AbstractElement first:FirstSetComputer.getFirstSet(element)){
   if(first instanceof Keyword)labels.add("keyword:"+((Keyword)first).getValue());
   else if(first instanceof RuleCall)labels.add("terminal:"+((RuleCall)first).getRule().getName());
   else throw new IllegalStateException(first.eClass().getName());
  }
  Collections.sort(labels);
  System.out.println("{\"case\":\""+name+"\",\"first\":["+String.join(",",labels.stream().map(s->"\""+s+"\"").toList())+"]}");
 }
 public static void main(String[] args){
  observe("sequence_stops_after_first",group(keyword("a"),keyword("b")));
  observe("nullable_prefix_includes_next",group(optional(keyword("a")),keyword("b")));
  observe("alternatives_deduplicate_keywords",choice(keyword("a"),keyword("b"),keyword("a")));
  observe("action_is_epsilon",group(F.createAction(),keyword("b")));
  ParserRule fragment=rule("FeatureSpecialization",choice(keyword(":"),keyword(":>"),keyword("typed")));fragment.setFragment(true);
  RuleCall reference=call(fragment);reference.setFirstSetPredicated(true);reference.setCardinality("+");
  observe("predicated_fragment_call",reference);
  RuleCall optionalCall=call(fragment);optionalCall.setCardinality("*");
  observe("repeat_call_does_not_add_epsilon_token",optionalCall);
  ParserRule nullable=rule("Nullable",optional(keyword("a")));
  observe("nullable_called_rule_includes_following",group(call(nullable),keyword("b")));
  TerminalRule id=F.createTerminalRule();id.setName("ID");id.setAlternatives(keyword("identifier"));
  Assignment assignment=F.createAssignment();assignment.setFeature("name");assignment.setOperator("=");assignment.setTerminal(call(id));
  observe("assignment_preserves_terminal_identity",assignment);
  CrossReference cross=F.createCrossReference();cross.setTerminal(call(id));observe("cross_reference_preserves_terminal_identity",cross);
 }
}
