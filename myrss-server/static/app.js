// Mark items as read functionality
document.addEventListener('DOMContentLoaded', function() {
    // Handle mark as read buttons
    const markReadButtons = document.querySelectorAll('.mark-read');
    
    markReadButtons.forEach(button => {
        button.addEventListener('click', async function() {
            const itemId = this.getAttribute('data-item-id');
            const article = this.closest('.item');
            
            try {
                const response = await fetch('/api/items/mark-read', {
                    method: 'POST',
                    headers: {
                        'Content-Type': 'application/json',
                    },
                    body: JSON.stringify({
                        item_ids: [itemId]
                    })
                });
                
                if (response.ok) {
                    article.classList.add('read');
                    this.remove();
                } else {
                    console.error('Failed to mark item as read');
                }
            } catch (error) {
                console.error('Error marking item as read:', error);
            }
        });
    });
    
    // Auto-mark items as read when clicking on links
    const itemLinks = document.querySelectorAll('.item-title a');
    
    itemLinks.forEach(link => {
        link.addEventListener('click', async function() {
            const article = this.closest('.item');
            const itemId = article.getAttribute('data-item-id');
            
            if (!article.classList.contains('read')) {
                try {
                    await fetch('/api/items/mark-read', {
                        method: 'POST',
                        headers: {
                            'Content-Type': 'application/json',
                        },
                        body: JSON.stringify({
                            item_ids: [itemId]
                        })
                    });
                    
                    article.classList.add('read');
                    const markReadBtn = article.querySelector('.mark-read');
                    if (markReadBtn) {
                        markReadBtn.remove();
                    }
                } catch (error) {
                    console.error('Error marking item as read:', error);
                }
            }
        });
    });
    
    // Form validation
    const addFeedForm = document.querySelector('.add-feed-form');
    
    if (addFeedForm) {
        addFeedForm.addEventListener('submit', function(e) {
            const urlInput = this.querySelector('#url');
            const contentInput = this.querySelector('#content');
            
            if (!urlInput.value.trim() && !contentInput.value.trim()) {
                e.preventDefault();
                alert('Please provide either a feed URL or RSS content');
            }
        });
    }
    
    // Refresh button indicator
    const refreshBtn = document.querySelector('.refresh-btn');
    
    if (refreshBtn) {
        refreshBtn.addEventListener('click', function(e) {
            this.textContent = 'Refreshing...';
            this.style.pointerEvents = 'none';
        });
    }
});

// Helper function for formatting relative dates
function formatRelativeDate(dateString) {
    const date = new Date(dateString);
    const now = new Date();
    const diff = now - date;
    
    const minutes = Math.floor(diff / 60000);
    const hours = Math.floor(diff / 3600000);
    const days = Math.floor(diff / 86400000);
    
    if (minutes < 1) return 'just now';
    if (minutes < 60) return `${minutes}m ago`;
    if (hours < 24) return `${hours}h ago`;
    if (days < 7) return `${days}d ago`;
    
    return date.toLocaleDateString('en-US', { 
        month: 'short', 
        day: 'numeric',
        year: date.getFullYear() !== now.getFullYear() ? 'numeric' : undefined
    });
}