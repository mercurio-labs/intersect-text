package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalGroupedMultiNegationLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalGroupedMultiNegationLexer() {;} 
    public InternalGroupedMultiNegationLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalGroupedMultiNegationLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalGroupedMultiNegation.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalGroupedMultiNegation.g:98:12: (~ (~ ( ( 'ab' | 'x' ) ) ) )
            // InternalGroupedMultiNegation.g:98:14: ~ (~ ( ( 'ab' | 'x' ) ) )
            {
            if ( input.LA(1)=='x' ) {
                input.consume();

            }
            else {
                MismatchedSetException mse = new MismatchedSetException(null,input);
                recover(mse);
                throw mse;}


            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    public void mTokens() throws RecognitionException {
        // InternalGroupedMultiNegation.g:1:8: ( RULE_VALUE )
        // InternalGroupedMultiNegation.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}