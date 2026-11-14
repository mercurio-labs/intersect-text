// Independent adapter to the actual Xtext predicate emitter; no production use.
package intersect.oracle;
import java.lang.reflect.*;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.generator.parser.antlr.*;
import org.eclipse.xtext.xtext.generator.grammarAccess.GrammarAccessExtensions;
public final class PredicateEmissionOracle extends AntlrGrammarGenerator {
 @Override protected boolean isCombinedGrammar(){return true;}
 public static void main(String[] args) throws Exception {
  PredicateEmissionOracle emitter=new PredicateEmissionOracle();
  Field access=AbstractAntlrGrammarGenerator.class.getDeclaredField("_grammarAccessExtensions");access.setAccessible(true);access.set(emitter,new GrammarAccessExtensions());
  for(String kind:new String[]{"first","full"}){
   Group group=XtextFactory.eINSTANCE.createGroup();
   for(String value:new String[]{"a","b"}){Keyword k=XtextFactory.eINSTANCE.createKeyword();k.setValue(value);group.getElements().add(k);}
   if(kind.equals("first"))group.setFirstSetPredicated(true);else group.setPredicated(true);
   System.out.println(kind+"\t"+emitter.ebnfPredicate(group,new AntlrOptions()).replace("\r","").replace("\n"," ").trim());
  }
 }
}
