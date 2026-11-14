// $ANTLR 3.2 Sep 23, 2009 12:02:23 D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g 2026-10-17 21:41:01

import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

public class NegationAudit21 extends Lexer {
    public static final int RULE_VALUE=4;
    public static final int EOF=-1;
    public static final int RULE_SHORT=5;
    public static final int RULE_REST=6;
     public int errors=0; @Override public void reportError(RecognitionException e) { errors++; } 

    // delegates
    // delegators

    public NegationAudit21() {;} 
    public NegationAudit21(CharStream input) {
        this(input, new RecognizerSharedState());
    }
    public NegationAudit21(CharStream input, RecognizerSharedState state) {
        super(input,state);

    }
    public String getGrammarFileName() { return "D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g"; }

    // $ANTLR start "RULE_VALUE"
    public final void mRULE_VALUE() throws RecognitionException {
        try {
            int _type = RULE_VALUE;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:3:11: ( 'a' ~ ( 'b' ) 'c' )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:3:13: 'a' ~ ( 'b' ) 'c'
            {
            match('a'); 
            if ( (input.LA(1)>='\u0000' && input.LA(1)<='a')||(input.LA(1)>='c' && input.LA(1)<='\uFFFF') ) {
                input.consume();

            }
            else {
                MismatchedSetException mse = new MismatchedSetException(null,input);
                recover(mse);
                throw mse;}

            match('c'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_VALUE"

    // $ANTLR start "RULE_SHORT"
    public final void mRULE_SHORT() throws RecognitionException {
        try {
            int _type = RULE_SHORT;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:4:11: ( 'a' )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:4:13: 'a'
            {
            match('a'); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_SHORT"

    // $ANTLR start "RULE_REST"
    public final void mRULE_REST() throws RecognitionException {
        try {
            int _type = RULE_REST;
            int _channel = DEFAULT_TOKEN_CHANNEL;
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:5:10: ( . )
            // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:5:12: .
            {
            matchAny(); 

            }

            state.type = _type;
            state.channel = _channel;
        }
        finally {
        }
    }
    // $ANTLR end "RULE_REST"

    public void mTokens() throws RecognitionException {
        // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:1:8: ( RULE_VALUE | RULE_SHORT | RULE_REST )
        int alt1=3;
        int LA1_0 = input.LA(1);

        if ( (LA1_0=='a') ) {
            int LA1_1 = input.LA(2);

            if ( ((LA1_1>='\u0000' && LA1_1<='a')||(LA1_1>='c' && LA1_1<='\uFFFF')) ) {
                alt1=1;
            }
            else {
                alt1=2;}
        }
        else if ( ((LA1_0>='\u0000' && LA1_0<='`')||(LA1_0>='b' && LA1_0<='\uFFFF')) ) {
            alt1=3;
        }
        else {
            NoViableAltException nvae =
                new NoViableAltException("", 1, 0, input);

            throw nvae;
        }
        switch (alt1) {
            case 1 :
                // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:1:10: RULE_VALUE
                {
                mRULE_VALUE(); 

                }
                break;
            case 2 :
                // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:1:21: RULE_SHORT
                {
                mRULE_SHORT(); 

                }
                break;
            case 3 :
                // D:\\dev\\repos\\mercurio-labs\\intersect\\target\\rcore-xtext-support-evidence\\terminal-negation-oracle-v1-record\\committed_path\\NegationAudit21.g:1:32: RULE_REST
                {
                mRULE_REST(); 

                }
                break;

        }

    }


 

}