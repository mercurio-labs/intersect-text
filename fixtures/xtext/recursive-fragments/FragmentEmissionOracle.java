// Intersect-authored adapter to the pinned Xtext fragment emitter; verification only.
package intersect.oracle;
import java.lang.reflect.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.generator.parser.antlr.*;
import org.eclipse.xtext.xtext.generator.grammarAccess.GrammarAccessExtensions;
public final class FragmentEmissionOracle extends AntlrGrammarGenerator {
 @Override protected boolean isCombinedGrammar(){return true;}
 static ParserRule rule(Grammar grammar,String name,EClass type,boolean fragment){
  ParserRule rule=XtextFactory.eINSTANCE.createParserRule();rule.setName(name);rule.setFragment(fragment);
  TypeRef reference=XtextFactory.eINSTANCE.createTypeRef();reference.setClassifier(type);rule.setType(reference);grammar.getRules().add(rule);return rule;
 }
 static Keyword keyword(String value){Keyword result=XtextFactory.eINSTANCE.createKeyword();result.setValue(value);return result;}
 static RuleCall call(ParserRule rule){RuleCall result=XtextFactory.eINSTANCE.createRuleCall();result.setRule(rule);return result;}
 public static void main(String[] args) throws Exception {
  FragmentEmissionOracle emitter=new FragmentEmissionOracle();
  for(Class<?> type=AntlrGrammarGenerator.class;type!=null;type=type.getSuperclass())for(Field field:type.getDeclaredFields())if(field.getType()==GrammarAccessExtensions.class){field.setAccessible(true);field.set(emitter,new GrammarAccessExtensions());}
  EClass feature=EcoreFactory.eINSTANCE.createEClass();feature.setName("Feature");
  Grammar grammar=XtextFactory.eINSTANCE.createGrammar();grammar.setName("example.FragmentOracle");
  ParserRule root=rule(grammar,"Root",feature,false),fragment=rule(grammar,"Tail",feature,true),node=rule(grammar,"Node",feature,false);
  Group rootBody=XtextFactory.eINSTANCE.createGroup();root.setAlternatives(rootBody);rootBody.getElements().add(keyword("start"));
  RuleCall outer=call(fragment);rootBody.getElements().add(outer);rootBody.getElements().add(keyword("end"));
  Group tail=XtextFactory.eINSTANCE.createGroup();fragment.setAlternatives(tail);tail.getElements().add(keyword("a"));RuleCall recursive=call(fragment);recursive.setCardinality("?");tail.getElements().add(recursive);
  Group nodeBody=XtextFactory.eINSTANCE.createGroup();node.setAlternatives(nodeBody);nodeBody.getElements().add(keyword("node"));RuleCall nested=call(fragment);nodeBody.getElements().add(nested);
  org.eclipse.xtext.xtext.RuleNames.ensureAdapterInstalled(grammar);
  for(var iterator=grammar.eAllContents();iterator.hasNext();){var item=iterator.next();if(item instanceof AbstractElement element)new org.eclipse.xtext.xtext.OriginalElement(element).attachToEmfObject(element);}
  for(RuleCall current:new RuleCall[]{outer,recursive,nested})System.out.println(emitter.ebnf2(current,new AntlrOptions(),true).replace("\r","").replace("\n"," ").replaceAll("\\s+"," ").trim());
 }
}
