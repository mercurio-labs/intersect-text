// Authored independent ANTLR 3.2.1 lexer oracle; verification only.
import org.antlr.runtime.*;
import java.util.Base64;
import java.nio.charset.StandardCharsets;
public class TerminalOracle {
 public static void main(String[] args) throws Exception {
  for (int i=1;i<args.length;i++) {
   String text=new String(Base64.getDecoder().decode(args[i]), StandardCharsets.UTF_8);
   Lexer lexer=(Lexer)Class.forName(args[0]).getConstructor(CharStream.class).newInstance(new ANTLRStringStream(text));
   Token token=lexer.nextToken();
   Token end=lexer.nextToken();
   int errors=lexer.getClass().getField("errors").getInt(lexer);
   boolean accepted=errors==0 && token.getType()!=Token.EOF && end.getType()==Token.EOF && text.equals(token.getText());
   System.out.println((i-1)+"\t"+accepted+"\t"+errors);
  }
 }
}
