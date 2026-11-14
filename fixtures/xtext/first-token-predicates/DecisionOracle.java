// Independently authored ANTLR decision probe; no production use.
import org.antlr.runtime.*;
public class DecisionOracle {
 public static void main(String[] args)throws Exception {
  for(String name:args)for(String text:new String[]{"a b","a c","a b a c","a b a b a c","a","b","a b c","a a c","a a b b","a a b c","a a a b b c"}) {
   Lexer lexer=(Lexer)Class.forName(name+"Lexer").getConstructor(CharStream.class).newInstance(new ANTLRStringStream(text));
   CommonTokenStream tokens=new CommonTokenStream(lexer);
   Parser parser=(Parser)Class.forName(name+"Parser").getConstructor(TokenStream.class).newInstance(tokens);
   parser.getClass().getMethod("entry").invoke(parser);
   System.out.println(name+"\t"+text+"\t"+parser.getClass().getField("branch").get(parser)+"\t"+parser.getNumberOfSyntaxErrors());
  }
 }
}
