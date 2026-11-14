package audit.parser.antlr.internal;

import org.eclipse.xtext.*;
import org.eclipse.xtext.parser.*;
import org.eclipse.xtext.parser.impl.*;
import org.eclipse.emf.ecore.util.EcoreUtil;
import org.eclipse.emf.ecore.EObject;
import org.eclipse.xtext.parser.antlr.AbstractInternalAntlrParser;
import org.eclipse.xtext.parser.antlr.XtextTokenStream;
import org.eclipse.xtext.parser.antlr.XtextTokenStream.HiddenTokens;
import org.eclipse.xtext.parser.antlr.AntlrDatatypeRuleToken;
import audit.services.ImportedUniqueGrammarAccess;



import org.antlr.runtime.*;
import java.util.Stack;
import java.util.List;
import java.util.ArrayList;

@SuppressWarnings("all")
public class InternalImportedUniqueParser extends AbstractInternalAntlrParser {
    public static final String[] tokenNames = new String[] {
        "<invalid>", "<EOR>", "<DOWN>", "<UP>", "RULE_ID", "RULE_WS", "'nodes'", "','", "';'", "'refs'"
    };
    public static final int T__9=9;
    public static final int T__8=8;
    public static final int T__7=7;
    public static final int RULE_ID=4;
    public static final int T__6=6;
    public static final int RULE_WS=5;
    public static final int EOF=-1;

    // delegates
    // delegators


        public InternalImportedUniqueParser(TokenStream input) {
            this(input, new RecognizerSharedState());
        }
        public InternalImportedUniqueParser(TokenStream input, RecognizerSharedState state) {
            super(input, state);
             
        }
        

    public String[] getTokenNames() { return InternalImportedUniqueParser.tokenNames; }
    public String getGrammarFileName() { return "InternalImportedUnique.g"; }



     	private ImportedUniqueGrammarAccess grammarAccess;

        public InternalImportedUniqueParser(TokenStream input, ImportedUniqueGrammarAccess grammarAccess) {
            this(input);
            this.grammarAccess = grammarAccess;
            registerRules(grammarAccess.getGrammar());
        }

        @Override
        protected String getFirstRuleName() {
        	return "Model";
       	}

       	@Override
       	protected ImportedUniqueGrammarAccess getGrammarAccess() {
       		return grammarAccess;
       	}




    // $ANTLR start "entryRuleModel"
    // InternalImportedUnique.g:64:1: entryRuleModel returns [EObject current=null] : iv_ruleModel= ruleModel EOF ;
    public final EObject entryRuleModel() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleModel = null;


        try {
            // InternalImportedUnique.g:64:46: (iv_ruleModel= ruleModel EOF )
            // InternalImportedUnique.g:65:2: iv_ruleModel= ruleModel EOF
            {
             newCompositeNode(grammarAccess.getModelRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleModel=ruleModel();

            state._fsp--;

             current =iv_ruleModel; 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleModel"


    // $ANTLR start "ruleModel"
    // InternalImportedUnique.g:71:1: ruleModel returns [EObject current=null] : (otherlv_0= 'nodes' ( (lv_nodes_1_0= ruleNode ) ) (otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) ) )* otherlv_4= ';' otherlv_5= 'refs' ( (otherlv_6= RULE_ID ) ) (otherlv_7= ',' ( (otherlv_8= RULE_ID ) ) )* otherlv_9= ';' ) ;
    public final EObject ruleModel() throws RecognitionException {
        EObject current = null;

        Token otherlv_0=null;
        Token otherlv_2=null;
        Token otherlv_4=null;
        Token otherlv_5=null;
        Token otherlv_6=null;
        Token otherlv_7=null;
        Token otherlv_8=null;
        Token otherlv_9=null;
        EObject lv_nodes_1_0 = null;

        EObject lv_nodes_3_0 = null;



        	enterRule();

        try {
            // InternalImportedUnique.g:77:2: ( (otherlv_0= 'nodes' ( (lv_nodes_1_0= ruleNode ) ) (otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) ) )* otherlv_4= ';' otherlv_5= 'refs' ( (otherlv_6= RULE_ID ) ) (otherlv_7= ',' ( (otherlv_8= RULE_ID ) ) )* otherlv_9= ';' ) )
            // InternalImportedUnique.g:78:2: (otherlv_0= 'nodes' ( (lv_nodes_1_0= ruleNode ) ) (otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) ) )* otherlv_4= ';' otherlv_5= 'refs' ( (otherlv_6= RULE_ID ) ) (otherlv_7= ',' ( (otherlv_8= RULE_ID ) ) )* otherlv_9= ';' )
            {
            // InternalImportedUnique.g:78:2: (otherlv_0= 'nodes' ( (lv_nodes_1_0= ruleNode ) ) (otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) ) )* otherlv_4= ';' otherlv_5= 'refs' ( (otherlv_6= RULE_ID ) ) (otherlv_7= ',' ( (otherlv_8= RULE_ID ) ) )* otherlv_9= ';' )
            // InternalImportedUnique.g:79:3: otherlv_0= 'nodes' ( (lv_nodes_1_0= ruleNode ) ) (otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) ) )* otherlv_4= ';' otherlv_5= 'refs' ( (otherlv_6= RULE_ID ) ) (otherlv_7= ',' ( (otherlv_8= RULE_ID ) ) )* otherlv_9= ';'
            {
            otherlv_0=(Token)match(input,6,FOLLOW_3); 

            			newLeafNode(otherlv_0, grammarAccess.getModelAccess().getNodesKeyword_0());
            		
            // InternalImportedUnique.g:83:3: ( (lv_nodes_1_0= ruleNode ) )
            // InternalImportedUnique.g:84:4: (lv_nodes_1_0= ruleNode )
            {
            // InternalImportedUnique.g:84:4: (lv_nodes_1_0= ruleNode )
            // InternalImportedUnique.g:85:5: lv_nodes_1_0= ruleNode
            {

            					newCompositeNode(grammarAccess.getModelAccess().getNodesNodeParserRuleCall_1_0());
            				
            pushFollow(FOLLOW_4);
            lv_nodes_1_0=ruleNode();

            state._fsp--;


            					if (current==null) {
            						current = createModelElementForParent(grammarAccess.getModelRule());
            					}
            					add(
            						current,
            						"nodes",
            						lv_nodes_1_0,
            						"audit.ImportedUnique.Node");
            					afterParserOrEnumRuleCall();
            				

            }


            }

            // InternalImportedUnique.g:102:3: (otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) ) )*
            loop1:
            do {
                int alt1=2;
                int LA1_0 = input.LA(1);

                if ( (LA1_0==7) ) {
                    alt1=1;
                }


                switch (alt1) {
            	case 1 :
            	    // InternalImportedUnique.g:103:4: otherlv_2= ',' ( (lv_nodes_3_0= ruleNode ) )
            	    {
            	    otherlv_2=(Token)match(input,7,FOLLOW_3); 

            	    				newLeafNode(otherlv_2, grammarAccess.getModelAccess().getCommaKeyword_2_0());
            	    			
            	    // InternalImportedUnique.g:107:4: ( (lv_nodes_3_0= ruleNode ) )
            	    // InternalImportedUnique.g:108:5: (lv_nodes_3_0= ruleNode )
            	    {
            	    // InternalImportedUnique.g:108:5: (lv_nodes_3_0= ruleNode )
            	    // InternalImportedUnique.g:109:6: lv_nodes_3_0= ruleNode
            	    {

            	    						newCompositeNode(grammarAccess.getModelAccess().getNodesNodeParserRuleCall_2_1_0());
            	    					
            	    pushFollow(FOLLOW_4);
            	    lv_nodes_3_0=ruleNode();

            	    state._fsp--;


            	    						if (current==null) {
            	    							current = createModelElementForParent(grammarAccess.getModelRule());
            	    						}
            	    						add(
            	    							current,
            	    							"nodes",
            	    							lv_nodes_3_0,
            	    							"audit.ImportedUnique.Node");
            	    						afterParserOrEnumRuleCall();
            	    					

            	    }


            	    }


            	    }
            	    break;

            	default :
            	    break loop1;
                }
            } while (true);

            otherlv_4=(Token)match(input,8,FOLLOW_5); 

            			newLeafNode(otherlv_4, grammarAccess.getModelAccess().getSemicolonKeyword_3());
            		
            otherlv_5=(Token)match(input,9,FOLLOW_3); 

            			newLeafNode(otherlv_5, grammarAccess.getModelAccess().getRefsKeyword_4());
            		
            // InternalImportedUnique.g:135:3: ( (otherlv_6= RULE_ID ) )
            // InternalImportedUnique.g:136:4: (otherlv_6= RULE_ID )
            {
            // InternalImportedUnique.g:136:4: (otherlv_6= RULE_ID )
            // InternalImportedUnique.g:137:5: otherlv_6= RULE_ID
            {

            					if (current==null) {
            						current = createModelElement(grammarAccess.getModelRule());
            					}
            				
            otherlv_6=(Token)match(input,RULE_ID,FOLLOW_4); 

            					newLeafNode(otherlv_6, grammarAccess.getModelAccess().getRefsNodeCrossReference_5_0());
            				

            }


            }

            // InternalImportedUnique.g:148:3: (otherlv_7= ',' ( (otherlv_8= RULE_ID ) ) )*
            loop2:
            do {
                int alt2=2;
                int LA2_0 = input.LA(1);

                if ( (LA2_0==7) ) {
                    alt2=1;
                }


                switch (alt2) {
            	case 1 :
            	    // InternalImportedUnique.g:149:4: otherlv_7= ',' ( (otherlv_8= RULE_ID ) )
            	    {
            	    otherlv_7=(Token)match(input,7,FOLLOW_3); 

            	    				newLeafNode(otherlv_7, grammarAccess.getModelAccess().getCommaKeyword_6_0());
            	    			
            	    // InternalImportedUnique.g:153:4: ( (otherlv_8= RULE_ID ) )
            	    // InternalImportedUnique.g:154:5: (otherlv_8= RULE_ID )
            	    {
            	    // InternalImportedUnique.g:154:5: (otherlv_8= RULE_ID )
            	    // InternalImportedUnique.g:155:6: otherlv_8= RULE_ID
            	    {

            	    						if (current==null) {
            	    							current = createModelElement(grammarAccess.getModelRule());
            	    						}
            	    					
            	    otherlv_8=(Token)match(input,RULE_ID,FOLLOW_4); 

            	    						newLeafNode(otherlv_8, grammarAccess.getModelAccess().getRefsNodeCrossReference_6_1_0());
            	    					

            	    }


            	    }


            	    }
            	    break;

            	default :
            	    break loop2;
                }
            } while (true);

            otherlv_9=(Token)match(input,8,FOLLOW_2); 

            			newLeafNode(otherlv_9, grammarAccess.getModelAccess().getSemicolonKeyword_7());
            		

            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleModel"


    // $ANTLR start "entryRuleNode"
    // InternalImportedUnique.g:175:1: entryRuleNode returns [EObject current=null] : iv_ruleNode= ruleNode EOF ;
    public final EObject entryRuleNode() throws RecognitionException {
        EObject current = null;

        EObject iv_ruleNode = null;


        try {
            // InternalImportedUnique.g:175:45: (iv_ruleNode= ruleNode EOF )
            // InternalImportedUnique.g:176:2: iv_ruleNode= ruleNode EOF
            {
             newCompositeNode(grammarAccess.getNodeRule()); 
            pushFollow(FOLLOW_1);
            iv_ruleNode=ruleNode();

            state._fsp--;

             current =iv_ruleNode; 
            match(input,EOF,FOLLOW_2); 

            }

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "entryRuleNode"


    // $ANTLR start "ruleNode"
    // InternalImportedUnique.g:182:1: ruleNode returns [EObject current=null] : ( (lv_name_0_0= RULE_ID ) ) ;
    public final EObject ruleNode() throws RecognitionException {
        EObject current = null;

        Token lv_name_0_0=null;


        	enterRule();

        try {
            // InternalImportedUnique.g:188:2: ( ( (lv_name_0_0= RULE_ID ) ) )
            // InternalImportedUnique.g:189:2: ( (lv_name_0_0= RULE_ID ) )
            {
            // InternalImportedUnique.g:189:2: ( (lv_name_0_0= RULE_ID ) )
            // InternalImportedUnique.g:190:3: (lv_name_0_0= RULE_ID )
            {
            // InternalImportedUnique.g:190:3: (lv_name_0_0= RULE_ID )
            // InternalImportedUnique.g:191:4: lv_name_0_0= RULE_ID
            {
            lv_name_0_0=(Token)match(input,RULE_ID,FOLLOW_2); 

            				newLeafNode(lv_name_0_0, grammarAccess.getNodeAccess().getNameIDTerminalRuleCall_0());
            			

            				if (current==null) {
            					current = createModelElement(grammarAccess.getNodeRule());
            				}
            				setWithLastConsumed(
            					current,
            					"name",
            					lv_name_0_0,
            					"audit.ImportedUnique.ID");
            			

            }


            }


            }


            	leaveRule();

        }

            catch (RecognitionException re) {
                recover(input,re);
                appendSkippedTokens();
            }
        finally {
        }
        return current;
    }
    // $ANTLR end "ruleNode"

    // Delegated rules


 

    public static final BitSet FOLLOW_1 = new BitSet(new long[]{0x0000000000000000L});
    public static final BitSet FOLLOW_2 = new BitSet(new long[]{0x0000000000000002L});
    public static final BitSet FOLLOW_3 = new BitSet(new long[]{0x0000000000000010L});
    public static final BitSet FOLLOW_4 = new BitSet(new long[]{0x0000000000000180L});
    public static final BitSet FOLLOW_5 = new BitSet(new long[]{0x0000000000000200L});

}