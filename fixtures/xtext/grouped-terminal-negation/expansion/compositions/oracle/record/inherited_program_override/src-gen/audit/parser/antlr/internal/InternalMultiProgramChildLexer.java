package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalMultiProgramChildLexer extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int RULE_PAIR=5;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalMultiProgramChildLexer() {;} 
    public InternalMultiProgramChildLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalMultiProgramChildLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalMultiProgramChild.g"; }

    // $ANTLR start "RULE_PAIR"
    public final void mRULE_PAIR() throws RecognitionException {
        try {
            // InternalMultiProgramChild.g:98:20: ( 'x' )
            // InternalMultiProgramChild.g:98:22: 'x'
            {
            match('x'); 

            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_PAIR"

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalMultiProgramChild.g:100:12: (~ ( RULE_PAIR ) )
            // InternalMultiProgramChild.g:100:14: ~ ( RULE_PAIR )
            {
            if ( (input.LA(1)>='\u0000' && input.LA(1)<='w')||(input.LA(1)>='y' && input.LA(1)<='\uFFFF') ) {
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
        // InternalMultiProgramChild.g:1:8: ( RULE_VALUE )
        // InternalMultiProgramChild.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}