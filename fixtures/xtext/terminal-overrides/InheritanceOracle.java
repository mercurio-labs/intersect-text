// Intersect-authored verification adapter to the pinned Xtext validator.
package intersect.oracle;
import java.util.*;
import org.eclipse.emf.ecore.EStructuralFeature;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.XtextValidator;
public final class InheritanceOracle extends XtextValidator {
 private final List<Integer> errors=new ArrayList<>();
 @Override protected void error(String message,EStructuralFeature feature,int index) {errors.add(index);}
 public static void main(String[] args) {
  var parser=new XtextStandaloneSetup().createInjectorAndDoEMFRegistration().getInstance(org.eclipse.xtext.parser.IParser.class);
  for(int count=0;count<=3;count++) {
   StringJoiner parents=new StringJoiner(",");for(int i=0;i<count;i++)parents.add("audit.Parent"+i);
   String source="grammar audit.Child"+(count==0?"":" with "+parents)+" terminal VALUE: 'a';";
   var result=parser.parse(new java.io.StringReader(source));
   if(result.hasSyntaxErrors())throw new IllegalStateException("Unexpected syntax rejection");
   Grammar grammar=(Grammar)result.getRootASTElement();
   for(int i=0;i<count;i++)grammar.getUsedGrammars().add(BindingOracle.grammar("audit.Parent"+i));
   InheritanceOracle validator=new InheritanceOracle();validator.checkGrammarUsesMaxOneOther(grammar);
   System.out.println("{\"parents\":"+count+",\"syntax_accepted\":true,\"error_indices\":"+validator.errors+"}");
  }
 }
}
