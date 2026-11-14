package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalFiniteUntilFragmentCompanionLexer extends Lexer {
    public static final int RULE_END=5;
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalFiniteUntilFragmentCompanionLexer() {;} 
    public InternalFiniteUntilFragmentCompanionLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalFiniteUntilFragmentCompanionLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalFiniteUntilFragmentCompanion.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalFiniteUntilFragmentCompanion.g:98:12: ( '@' ( options {greedy=false; } : . )* RULE_END )
            // InternalFiniteUntilFragmentCompanion.g:98:14: '@' ( options {greedy=false; } : . )* RULE_END
            {
            match('@'); 
            // InternalFiniteUntilFragmentCompanion.g:98:18: ( options {greedy=false; } : . )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0=='!') ) {
                    alt1=2;
                }
                else if ( ((LA1_0>='\u0000' && LA1_0<=' ')||(LA1_0>='\"' && LA1_0<='\uFFFF')) ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalFiniteUntilFragmentCompanion.g:98:46: .
            	    {
            	    matchAny(); 

            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);

            mRULE_END(); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_END"
    public final void mRULE_END() throws RecognitionException {
        try {
            // InternalFiniteUntilFragmentCompanion.g:100:19: ( '!' )
            // InternalFiniteUntilFragmentCompanion.g:100:21: '!'
            {
            match('!'); 

            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_END"

    public void mTokens() throws RecognitionException {
        // InternalFiniteUntilFragmentCompanion.g:1:8: ( RULE_VALUE )
        // InternalFiniteUntilFragmentCompanion.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}