// Independent adapter to the pinned Xtext enum emitter; never production code.
package intersect.oracle;
import java.lang.reflect.*;
import org.eclipse.emf.ecore.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.generator.parser.antlr.*;
import org.eclipse.xtext.xtext.generator.grammarAccess.GrammarAccessExtensions;
public final class EnumEmissionOracle extends AntlrGrammarGenerator {
 @Override protected boolean isCombinedGrammar(){return true;}
 public static void main(String[] args) throws Exception {
  EnumEmissionOracle emitter=new EnumEmissionOracle();
  for(Class<?> type=AntlrGrammarGenerator.class;type!=null;type=type.getSuperclass()) {
   for(Field field:type.getDeclaredFields())if(field.getType()==GrammarAccessExtensions.class){field.setAccessible(true);field.set(emitter,new GrammarAccessExtensions());}
  }
  EEnum enumeration=EcoreFactory.eINSTANCE.createEEnum();enumeration.setName("VisibilityKind");
  for(String name:new String[]{"private","protected","public","publicAlias"}){EEnumLiteral literal=EcoreFactory.eINSTANCE.createEEnumLiteral();literal.setName(name);literal.setValue(Math.min(enumeration.getELiterals().size(),2));enumeration.getELiterals().add(literal);}
  Grammar grammar=XtextFactory.eINSTANCE.createGrammar();grammar.setName("example.EnumOracle");
  EnumRule rule=XtextFactory.eINSTANCE.createEnumRule();rule.setName("VisibilityIndicator");grammar.getRules().add(rule);
  TypeRef type=XtextFactory.eINSTANCE.createTypeRef();type.setClassifier(enumeration);rule.setType(type);
  Alternatives alternatives=XtextFactory.eINSTANCE.createAlternatives();rule.setAlternatives(alternatives);
  String[][] cases={{"public","public"},{"private","["},{"protected","protected"},{"publicAlias","alias"},{"public","+"}};
  for(String[] row:cases){EnumLiteralDeclaration declaration=XtextFactory.eINSTANCE.createEnumLiteralDeclaration();declaration.setEnumLiteral(enumeration.getEEnumLiteral(row[0]));Keyword keyword=XtextFactory.eINSTANCE.createKeyword();keyword.setValue(row[1]);declaration.setLiteral(keyword);alternatives.getElements().add(declaration);}
  org.eclipse.xtext.xtext.RuleNames.ensureAdapterInstalled(grammar);
  for(var iterator=grammar.eAllContents();iterator.hasNext();){var item=iterator.next();if(item instanceof AbstractElement element)new org.eclipse.xtext.xtext.OriginalElement(element).attachToEmfObject(element);}
  for(AbstractElement item:alternatives.getElements()){
   EnumLiteralDeclaration declaration=(EnumLiteralDeclaration)item;
   String emitted=emitter.ebnf2(declaration,new AntlrOptions(),true).replace("\r","").replace("\n"," ").replaceAll("\\s+"," ").trim();
   System.out.println(declaration.getLiteral().getValue()+"\t"+declaration.getEnumLiteral().getName()+"\t"+enumeration.getELiterals().indexOf(declaration.getEnumLiteral())+"\t"+emitted);
  }
 }
}
