package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFirstRuleEnumLibraryChildLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalFirstRuleEnumLibraryChildLexer() {;} 
    public InternalFirstRuleEnumLibraryChildLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalFirstRuleEnumLibraryChildLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalFirstRuleEnumLibraryChild.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalFirstRuleEnumLibraryChild.g:99:12: ( 'x' )
            // InternalFirstRuleEnumLibraryChild.g:99:14: 'x'
            {
            match('x'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    public void mTokens() throws RecognitionException {
        // InternalFirstRuleEnumLibraryChild.g:1:8: ( RULE_VALUE )
        // InternalFirstRuleEnumLibraryChild.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}