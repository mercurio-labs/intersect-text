package audit.parser.antlr.internal;

// Hack: Use our own Lexer superclass by means of import. 
// Currently there is no other way to specify the superclass for the lexer.
import org.eclipse.xtext.parser.antlr.Lexer;


import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalUntilCallsOrdinaryFragmentSuffixCLexer extends Lexer {
    public static final int RULE_END=5;
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;

    // delegates
    // delegators

    public InternalUntilCallsOrdinaryFragmentSuffixCLexer() {;} 
    public InternalUntilCallsOrdinaryFragmentSuffixCLexer(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public InternalUntilCallsOrdinaryFragmentSuffixCLexer(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "InternalUntilCallsOrdinaryFragmentSuffixC.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // InternalUntilCallsOrdinaryFragmentSuffixC.g:98:12: ( '@' RULE_END 'c' )
            // InternalUntilCallsOrdinaryFragmentSuffixC.g:98:14: '@' RULE_END 'c'
            {
            match('@'); 
            mRULE_END(); 
            match('c'); 

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
            // InternalUntilCallsOrdinaryFragmentSuffixC.g:100:19: ( ( 'ab' | 'abc' ) )
            // InternalUntilCallsOrdinaryFragmentSuffixC.g:100:21: ( 'ab' | 'abc' )
            {
            // InternalUntilCallsOrdinaryFragmentSuffixC.g:100:21: ( 'ab' | 'abc' )
            int alt1=2;
            int LA1_0 = input.LA(1);

            if ( (LA1_0=='a') ) {
                int LA1_1 = input.LA(2);

                if ( (LA1_1=='b') ) {
                    int LA1_2 = input.LA(3);

                    if ( (LA1_2=='c') ) {
                        alt1=2;
                    }
                    else {
                        alt1=1;}
                }
                else {
                    NoViableAltException nvae =
                        new NoViableAltException("", 1, 1, input);

                    throw nvae;
                }
            }
            else {
                NoViableAltException nvae =
                    new NoViableAltException("", 1, 0, input);

                throw nvae;
            }
            switch (alt1) {
                case 1 :
                    // InternalUntilCallsOrdinaryFragmentSuffixC.g:100:22: 'ab'
                    {
                    match("ab"); 


                    }
                    break;
                case 2 :
                    // InternalUntilCallsOrdinaryFragmentSuffixC.g:100:27: 'abc'
                    {
                    match("abc"); 


                    }
                    break;

            }


            }

        }
        finally {
        }
    }
    // $ANTLR end "RULE_END"

    public void mTokens() throws RecognitionException {
        // InternalUntilCallsOrdinaryFragmentSuffixC.g:1:8: ( RULE_VALUE )
        // InternalUntilCallsOrdinaryFragmentSuffixC.g:1:10: RULE_VALUE
        {
        mRULE_VALUE(); 

        }


    }


 

}