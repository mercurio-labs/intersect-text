// Authored verification adapter; never used in production.
package intersect.oracle;
import org.eclipse.xtext.*;
import org.eclipse.xtext.xtext.RuleNames;
import org.eclipse.xtext.xtext.generator.parser.antlr.AntlrGrammarGenUtil;
import org.antlr.runtime.*;
public final class TerminalIdentityOracle {
 public static void main(String[] args) throws Exception {
  if(args[0].equals("names")) {
   Grammar base=BindingOracle.grammar("audit.Base"), child=BindingOracle.grammar("audit.Child");
   child.getUsedGrammars().add(base);
   TerminalRule parent=BindingOracle.terminal(base,"Choice",false,"red");
   TerminalRule replacement=BindingOracle.terminal(child,"Choice",false,"red");
   ParserRule model=XtextFactory.eINSTANCE.createParserRule();model.setName("Model");
   Group group=XtextFactory.eINSTANCE.createGroup();group.getElements().add(BindingOracle.call(replacement,false));group.getElements().add(BindingOracle.call(parent,true));model.setAlternatives(group);child.getRules().add(0,model);
   new RuleNames(child,true);
   java.util.List<TerminalRule> rules=GrammarUtil.allTerminalRules(child);
   if(rules.size()!=2 || rules.get(0)!=replacement || rules.get(1)!=parent)throw new AssertionError("Unexpected terminal order");
   System.out.println("{\"child\":\""+AntlrGrammarGenUtil.getRuleName(replacement)+"\",\"parent\":\""+AntlrGrammarGenUtil.getRuleName(parent)+"\",\"order\":[\"child\",\"parent\"]}");
  } else {
   Class<?> type=Class.forName(args[1]);
   Lexer lexer=(Lexer)type.getConstructor(CharStream.class).newInstance(new ANTLRStringStream(args[4]));
   Token token=lexer.nextToken(),end=lexer.nextToken();
   String selected=token.getType()==type.getField(args[2]).getInt(null)?"child":token.getType()==type.getField(args[3]).getInt(null)?"parent":"other";
   System.out.println("{\"selected\":\""+selected+"\",\"complete\":"+(end.getType()==Token.EOF && args[4].equals(token.getText()))+"}");
  }
 }
}
