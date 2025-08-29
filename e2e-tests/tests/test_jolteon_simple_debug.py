from pytest import mark
from src.blockchain_api import BlockchainApi, Wallet
from config.api_config import ApiConfig
import logging as logger
import json


class TestJolteonSimpleDebug:
    """Very simple debug tests to understand Jolteon block structure"""
    
    @mark.test_key('JOLTEON-SIMPLE-DEBUG-001')
    def test_raw_block_dump(self, api: BlockchainApi, config: ApiConfig):
        """Simply dump the raw block structure to see what's available"""
        logger.info("Starting raw block structure dump")
        
        try:
            # Get a recent block
            block_info = api.substrate.get_block()
            
            logger.info("=== RAW BLOCK STRUCTURE DUMP ===")
            logger.info(f"Block info type: {type(block_info)}")
            logger.info(f"Block info keys: {list(block_info.keys()) if isinstance(block_info, dict) else 'Not a dict'}")
            
            # Pretty print the entire structure
            logger.info("=== COMPLETE BLOCK STRUCTURE ===")
            logger.info(json.dumps(block_info, indent=2, default=str))
            
        except Exception as e:
            logger.error(f"Error getting block: {e}")
            raise
        
        # Always pass this test as it's exploratory
        assert True, "Raw block dump completed"

    @mark.test_key('JOLTEON-SIMPLE-DEBUG-002')
    def test_header_only_dump(self, api: BlockchainApi, config: ApiConfig):
        """Dump just the header structure"""
        logger.info("Starting header structure dump")
        
        try:
            # Get a recent block
            block_info = api.substrate.get_block()
            
            if 'header' in block_info:
                header = block_info['header']
                logger.info("=== HEADER STRUCTURE DUMP ===")
                logger.info(f"Header type: {type(header)}")
                logger.info(f"Header keys: {list(header.keys())}")
                logger.info(json.dumps(header, indent=2, default=str))
            else:
                logger.info("No header found in block")
                
        except Exception as e:
            logger.error(f"Error getting header: {e}")
            raise
        
        # Always pass this test as it's exploratory
        assert True, "Header dump completed"

    @mark.test_key('JOLTEON-SIMPLE-DEBUG-003')
    def test_digest_analysis(self, api: BlockchainApi, config: ApiConfig):
        """Analyze digest items specifically"""
        logger.info("Starting digest analysis")
        
        try:
            # Get a recent block
            block_info = api.substrate.get_block()
            
            if 'header' in block_info and 'digest' in block_info['header']:
                digest = block_info['header']['digest']
                logger.info("=== DIGEST ANALYSIS ===")
                logger.info(f"Digest type: {type(digest)}")
                logger.info(f"Digest length: {len(digest)}")
                
                for i, item in enumerate(digest):
                    logger.info(f"Digest item {i}:")
                    logger.info(f"  Type: {type(item)}")
                    logger.info(f"  Content: {item}")
                    
                    if isinstance(item, dict):
                        logger.info(f"  Keys: {list(item.keys())}")
                        for key, value in item.items():
                            logger.info(f"    {key}: {value}")
            else:
                logger.info("No digest found in header")
                
        except Exception as e:
            logger.error(f"Error analyzing digest: {e}")
            raise
        
        # Always pass this test as it's exploratory
        assert True, "Digest analysis completed"


