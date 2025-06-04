// Mark items as read
document.addEventListener('DOMContentLoaded', function() {
    // Handle mark as read buttons
    document.querySelectorAll('.mark-read-btn').forEach(button => {
        button.addEventListener('click', async function() {
            const itemId = this.dataset.itemId;
            const itemElement = document.querySelector(`[data-item-id="${itemId}"]`);
            
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
                    itemElement.classList.add('read');
                    this.remove();
                }
            } catch (error) {
                console.error('Failed to mark item as read:', error);
            }
        });
    });
    
    // Handle edit labels buttons
    document.querySelectorAll('.edit-labels-btn').forEach(button => {
        button.addEventListener('click', function() {
            const subscriptionId = this.dataset.subscriptionId;
            const modal = document.getElementById('label-edit-modal');
            const form = document.getElementById('label-edit-form');
            
            // Set form action
            form.action = `/feeds/${subscriptionId}/labels`;
            
            // Get current labels for this subscription
            const subscriptionItem = this.closest('.subscription-item');
            const currentLabels = Array.from(subscriptionItem.querySelectorAll('.labels .label'))
                .map(label => label.textContent.trim());
            
            // Check the appropriate checkboxes
            form.querySelectorAll('input[type="checkbox"]').forEach(checkbox => {
                checkbox.checked = currentLabels.includes(checkbox.value);
            });
            
            // Show modal
            modal.style.display = 'flex';
        });
    });
    
    // Handle form submission for label editing
    const labelEditForm = document.getElementById('label-edit-form');
    if (labelEditForm) {
        labelEditForm.addEventListener('submit', function(e) {
            e.preventDefault();
            
            const formData = new FormData(this);
            const selectedLabels = Array.from(formData.getAll('labels'));
            
            // Add new labels from the text input
            const newLabelsInput = document.getElementById('new-labels');
            if (newLabelsInput && newLabelsInput.value) {
                const newLabels = newLabelsInput.value.split(',').map(label => label.trim()).filter(label => label);
                selectedLabels.push(...newLabels);
            }
            
            // Create a new form data with the combined labels
            const submitData = new FormData();
            selectedLabels.forEach(label => submitData.append('labels', label));
            
            // Submit the form
            fetch(this.action, {
                method: 'POST',
                body: submitData
            }).then(response => {
                if (response.ok) {
                    window.location.reload();
                }
            });
        });
    }
    
    // Handle labels field in add feed form
    const addFeedForm = document.querySelector('.add-feed-form');
    if (addFeedForm) {
        addFeedForm.addEventListener('submit', function(e) {
            const labelsInput = this.querySelector('input[name="labels"]');
            if (labelsInput && labelsInput.value) {
                const labels = labelsInput.value.split(',').map(label => label.trim()).filter(label => label);
                
                // Clear the original input
                labelsInput.removeAttribute('name');
                
                // Add hidden inputs for each label
                labels.forEach(label => {
                    const input = document.createElement('input');
                    input.type = 'hidden';
                    input.name = 'labels';
                    input.value = label;
                    this.appendChild(input);
                });
            }
        });
    }
});

// Close modal function
function closeModal() {
    const modal = document.getElementById('label-edit-modal');
    if (modal) {
        modal.style.display = 'none';
    }
}

// Close modal when clicking outside
window.addEventListener('click', function(e) {
    const modal = document.getElementById('label-edit-modal');
    if (e.target === modal) {
        closeModal();
    }
});